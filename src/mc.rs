//! The core of the Monte Carlo simulator.

use crate::capture::{Capture, CaptureMut};
use crate::probs::{Fraction, SliceExt};
use crate::selection::Selection;
use std::ops::DerefMut;
use tinyrand::{Rand, StdRand};
use crate::linear::Matrix;

pub struct MonteCarloEngine<'a, R: Rand> {
    iterations: u64,
    probs: Option<Capture<'a, Matrix, Matrix>>,
    podium_places: Option<usize>,
    dilatives: Option<Capture<'a, Vec<f64>, [f64]>>,
    win_probs: Option<Capture<'a, Vec<f64>, [f64]>>,
    podium: Option<CaptureMut<'a, Vec<usize>, [usize]>>,
    bitmap: Option<CaptureMut<'a, Vec<bool>, [bool]>>,
    rand: CaptureMut<'a, R, R>,
}
impl<'a, R: Rand> MonteCarloEngine<'a, R> {
    pub fn new(rand: CaptureMut<'a, R, R>) -> Self where R: Default {
        Self {
            iterations: 10_000,
            probs: None,
            podium_places: None,
            dilatives: None,
            win_probs: None,
            podium: None,
            bitmap: None,
            rand,
        }
    }

    pub fn with_iterations(mut self, iterations: u64) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_probs(mut self, probs: Capture<'a, Matrix, Matrix>) -> Self {
        self.probs = Some(probs);
        self
    }

    pub fn with_podium_places(mut self, places: usize) -> Self {
        self.podium_places = Some(places);
        self
    }

    pub fn with_dilatives(mut self, dilatives: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.dilatives = Some(dilatives);
        self
    }

    pub fn with_win_probs(mut self, win_probs: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.win_probs = Some(win_probs);
        self
    }

    pub fn with_podium(mut self, podium: CaptureMut<'a, Vec<usize>, [usize]>) -> Self {
        self.podium = Some(podium);
        self
    }

    pub fn with_bitmap(mut self, bitmap: CaptureMut<'a, Vec<bool>, [bool]>) -> Self {
        self.bitmap = Some(bitmap);
        self
    }

    fn num_runners(&self) -> usize {
        if let Some(probs) = &self.probs {
            probs.cols()
        } else if let Some(win_probs) = &self.win_probs {
            win_probs.len()
        } else {
            panic!("no probabilities specified");
        }
    }

    fn num_ranks(&self) -> usize {
        if let Some(podium_places) = self.podium_places {
            podium_places
        } else if let Some(podium) = &self.podium {
            podium.len()
        } else if let Some(probs) = &self.probs {
            probs.rows()
        } else if let Some(dilatives) = &self.dilatives {
            dilatives.len()
        } else {
            panic!("no podium specified");
        }
    }

    fn validate_state(&self) {
        todo!()
    }

    pub fn simulate(&mut self, selections: &[Selection]) -> Fraction {
        if self.bitmap.is_none() {
            self.bitmap = Some(CaptureMut::Owned(vec![true; self.num_runners()]));
        }
        if self.podium.is_none() {
            self.podium = Some(CaptureMut::Owned(vec![usize::MAX; self.num_ranks()]))
        }

        if self.probs.is_none() {
            let mut probs = Matrix::allocate(self.num_ranks(), self.num_runners());
            probs.clone_row(&*self.win_probs.as_ref().unwrap());
            if let Some(dilatives) = &self.dilatives {
                dilatives.dilate_rows_additive(&mut probs);
            }
            self.probs = Some(Capture::Owned(probs));
        }

        println!("simulating with: \n{}", self.probs.as_ref().unwrap().verbose());

        run_many(
            self.iterations,
            selections,
            self.probs.as_ref().unwrap(),
            self.podium.as_mut().unwrap(),
            self.bitmap.as_mut().unwrap(),
            self.rand.deref_mut(),
        )
    }
}

impl Default for MonteCarloEngine<'_, StdRand> {
    fn default() -> Self {
        Self::new(CaptureMut::Owned(StdRand::default()))
    }
}

pub struct DilatedProbs<'a> {
    win_probs: Option<Capture<'a, Vec<f64>, [f64]>>,
    dilatives: Option<Capture<'a, Vec<f64>, [f64]>>,
}
impl<'a> DilatedProbs<'a> {
    pub fn with_win_probs(mut self, win_probs: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.win_probs = Some(win_probs);
        self
    }

    pub fn with_dilatives(mut self, dilatives: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.dilatives = Some(dilatives);
        self
    }

    pub fn undilated(self, podium_places: usize) -> Self {
        self.with_dilatives(Capture::Owned(vec![0.0; podium_places]))
    }
}

impl Default for DilatedProbs<'_> {
    fn default() -> Self {
        Self {
            win_probs: None,
            dilatives: None
        }
    }
}

impl From<DilatedProbs<'_>> for Matrix {
    fn from(probs: DilatedProbs) -> Self {
        let win_probs = probs.win_probs.expect("no win probabilities specified");
        let dilatives = probs.dilatives.expect("no dilatives specified");
        let mut matrix = Matrix::allocate(dilatives.len(), win_probs.len());
        matrix.clone_row(&win_probs);
        dilatives.dilate_rows_additive(&mut matrix);
        matrix
    }
}

pub fn run_many(iterations: u64, selections: &[Selection], probs: &Matrix, podium: &mut [usize], bitmap: &mut [bool], rand: &mut impl Rand,) -> Fraction {
    assert!(validate_args(probs, podium, bitmap));

    let mut matching_iters = 0;
    for _ in 0..iterations {
        run_once(probs, podium, bitmap, rand);
        let mut all_match = true;
        for selection in selections {
            if !selection.matches(podium) {
                all_match = false;
                break;
            }
        }
        if all_match {
            matching_iters += 1;
        }
    }
    Fraction {
        numerator: matching_iters,
        denominator: iterations,
    }
}

#[inline(always)]
pub fn run_once(
    probs: &Matrix,
    podium: &mut [usize],
    bitmap: &mut [bool],
    rand: &mut impl Rand,
) {
    debug_assert!(validate_args(probs, podium, bitmap));

    let runners = probs.cols();
    let mut prob_sum = 1.0;
    reset_bitmap(bitmap);
    // println!("podium.len: {}", podium.len());
    for (rank, ranked_runner) in podium.iter_mut().enumerate() {
        let mut cumulative = 0.0;
        let random = random_f64(rand) * prob_sum;
        let rank_probs = probs.row_slice(rank);
        // println!("random={random:.3}, prob_sum={prob_sum}");
        for runner in 0..runners {
            if bitmap[runner] {
                let prob = rank_probs[runner];
                cumulative += prob;
                // println!("probabilities[{runner}]={prob:.3}, cumulative={cumulative:.3}");
                if cumulative >= random {
                    // println!("chosen runner {runner} for rank {rank}");
                    *ranked_runner = runner;
                    bitmap[runner] = false;
                    prob_sum -= prob;
                    break;
                }
            } /*else {
                  println!("skipping runner {runner}");
              }*/
        }
    }

    // println!("podium: {podium:?}");
}

fn validate_args(probs: &Matrix, podium: &mut [usize], bitmap: &mut [bool]) -> bool {
    assert!(
        !probs.is_empty(),
        "the probabilities matrix cannot be empty"
    );
    assert_eq!(
        probs.cols(),
        bitmap.len(),
        "a bitmap entry must exist for each runner"
    );
    assert_eq!(
        probs.rows(),
        podium.len(),
        "a probability row must exist for each podium entry"
    );
    assert!(!podium.is_empty(), "the podium slice cannot be empty");
    assert!(
        podium.len() <= probs.cols(),
        "number of podium entries cannot exceed number of runners"
    );
    for &p in probs.flatten() {
        assert!(p >= 0.0 && p <= 1.0, "probabilities out of range: {probs}");
    }
    true
}

#[inline(always)]
fn reset_bitmap(bitmap: &mut [bool]) {
    for b in bitmap {
        *b = true;
    }
}

#[inline(always)]
fn random_f64(rand: &mut impl Rand) -> f64 {
    rand.next_u64() as f64 / u64::MAX as f64
}
