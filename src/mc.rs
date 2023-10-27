//! The core of the Monte Carlo simulator.

use crate::capture::{Capture, CaptureMut};
use crate::linear::Matrix;
use crate::probs::{Fraction, SliceExt};
use crate::selection::{Selection, Selections};
use std::ops::DerefMut;
use tinyrand::{Rand, StdRand};

pub struct MonteCarloEngine<'a, R: Rand> {
    iterations: u64,
    probs: Option<Capture<'a, Matrix<f64>, Matrix<f64>>>,
    podium: Option<CaptureMut<'a, Vec<usize>, [usize]>>,
    bitmap: Option<CaptureMut<'a, Vec<bool>, [bool]>>,
    totals: Option<CaptureMut<'a, Vec<f64>, [f64]>>,
    rand: CaptureMut<'a, R, R>,
}
impl<'a, R: Rand> MonteCarloEngine<'a, R> {
    pub fn new(rand: CaptureMut<'a, R, R>) -> Self
    where
        R: Default,
    {
        Self {
            iterations: 10_000,
            probs: None,
            podium: None,
            bitmap: None,
            totals: None,
            rand,
        }
    }

    pub fn reset_rand(&mut self) where R: Default {
        self.set_rand(R::default());
    }

    pub fn set_rand(&mut self, rand: R) {
        self.rand = rand.into();
    }

    pub fn with_iterations(mut self, iterations: u64) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_probs(mut self, probs: Capture<'a, Matrix<f64>, Matrix<f64>>) -> Self {
        self.probs = Some(probs);
        self
    }

    pub fn set_probs(&mut self, probs: Capture<'a, Matrix<f64>, Matrix<f64>>) {
        self.probs = Some(probs);
    }

    pub fn probs(&self) -> Option<&Capture<'a, Matrix<f64>, Matrix<f64>>> {
        self.probs.as_ref()
    }

    pub fn with_podium(mut self, podium: CaptureMut<'a, Vec<usize>, [usize]>) -> Self {
        self.podium = Some(podium);
        self
    }

    pub fn with_bitmap(mut self, bitmap: CaptureMut<'a, Vec<bool>, [bool]>) -> Self {
        self.bitmap = Some(bitmap);
        self
    }

    pub fn with_totals(mut self, totals: CaptureMut<'a, Vec<f64>, [f64]>) -> Self {
        self.totals = Some(totals);
        self
    }

    fn num_runners(&self) -> usize {
        if let Some(probs) = &self.probs {
            probs.cols()
        } else {
            panic!("no probabilities specified");
        }
    }

    fn num_ranks(&self) -> usize {
        if let Some(podium) = &self.podium {
            podium.len()
        } else if let Some(probs) = &self.probs {
            probs.rows()
        } else {
            panic!("no podium specified");
        }
    }

    pub fn simulate(&mut self, selections: &[Selection]) -> Fraction {
        self.ensure_init();
        // println!("simulating with: \n{}", self.probs.as_ref().unwrap().verbose());

        simulate(
            self.iterations,
            selections,
            self.probs.as_ref().unwrap(),
            self.podium.as_mut().unwrap(),
            self.bitmap.as_mut().unwrap(),
            self.totals.as_mut().unwrap(),
            self.rand.deref_mut(),
        )
    }

    pub fn simulate_batch(
        &mut self,
        scenarios: &[Selections],
        counts: &mut [u64],
    ) {
        self.ensure_init();
        // println!("simulating with: \n{}", self.probs.as_ref().unwrap().verbose());

        simulate_batch(
            self.iterations,
            scenarios,
            counts,
            self.probs.as_ref().unwrap(),
            self.podium.as_mut().unwrap(),
            self.bitmap.as_mut().unwrap(),
            self.totals.as_mut().unwrap(),
            self.rand.deref_mut(),
        )
    }

    fn ensure_init(&mut self) {
        if self.bitmap.is_none() {
            self.bitmap = Some(CaptureMut::Owned(vec![true; self.num_runners()]));
        }
        if self.podium.is_none() {
            self.podium = Some(CaptureMut::Owned(vec![usize::MAX; self.num_ranks()]))
        }
        if self.totals.is_none() {
            self.totals = Some(CaptureMut::Owned(vec![1.0; self.num_ranks()]))
        }
    }
}

impl Default for MonteCarloEngine<'_, StdRand> {
    fn default() -> Self {
        Self::new(CaptureMut::Owned(StdRand::default()))
    }
}

#[derive(Default)]
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

    pub fn with_podium_places(self, podium_places: usize) -> Self {
        self.with_dilatives(Capture::Owned(vec![0.0; podium_places]))
    }
}

impl From<DilatedProbs<'_>> for Matrix<f64> {
    fn from(probs: DilatedProbs) -> Self {
        let win_probs = probs.win_probs.expect("no win probabilities specified");
        let dilatives = probs.dilatives.expect("no dilatives specified");
        let mut matrix = Matrix::allocate(dilatives.len(), win_probs.len());
        matrix.clone_row(&win_probs);
        dilatives.dilate_rows_power(&mut matrix);
        matrix
    }
}

pub fn simulate_batch(
    iterations: u64,
    scenarios: &[Selections],
    counts: &mut [u64],
    probs: &Matrix<f64>,
    podium: &mut [usize],
    bitmap: &mut [bool],
    totals: &mut [f64],
    rand: &mut impl Rand,
) {
    assert!(validate_args(probs, podium, bitmap, totals));
    assert_eq!(
        scenarios.len(),
        counts.len(),
        "a count must exist for each scenario"
    );

    counts.fill(0);
    for _ in 0..iterations {
        run_once(probs, podium, bitmap, totals, rand);
        for (scenario_index, selections) in scenarios.iter().enumerate() {
            if selections.iter().all(|selection| selection.matches(podium)) {
                counts[scenario_index] += 1;
            }
        }
    }
}

pub fn simulate(
    iterations: u64,
    selections: &[Selection],
    probs: &Matrix<f64>,
    podium: &mut [usize],
    bitmap: &mut [bool],
    totals: &mut [f64],
    rand: &mut impl Rand,
) -> Fraction {
    assert!(validate_args(probs, podium, bitmap, totals));

    let mut matching_iters = 0;
    for _ in 0..iterations {
        run_once(probs, podium, bitmap, totals, rand);
        if selections.iter().all(|selection| selection.matches(podium)) {
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
    probs: &Matrix<f64>,
    podium: &mut [usize],
    bitmap: &mut [bool],
    totals: &mut [f64],
    rand: &mut impl Rand,
) {
    debug_assert!(validate_args(probs, podium, bitmap, totals));
    bitmap.fill(true);
    totals.fill(1.0);

    let runners = probs.cols();
    let ranks = podium.len();
    // reset_bitmap(bitmap);
    // println!("podium.len: {}", podium.len());
    for (rank, ranked_runner) in podium.iter_mut().enumerate() {
        let mut cumulative = 0.0;

        let rank_probs = probs.row_slice(rank);
        let total = totals[rank];
        let random = random_f64(rand) * total;
        // println!("random={random:.3}, prob_sum={prob_sum}");
        let mut chosen = false;
        for runner in 0..runners {
            if bitmap[runner] {
                let prob = rank_probs[runner];
                cumulative += prob;
                // println!("probabilities[{runner}]={prob:.3}, cumulative={cumulative:.3}");
                if cumulative >= random {
                    // println!("chosen runner {runner} for rank {rank}");
                    *ranked_runner = runner;
                    bitmap[runner] = false;
                    chosen = true;
                    for future_rank in rank + 1..ranks {
                        totals[future_rank] -= probs[(future_rank, runner)];
                    }
                    break;
                }
            } /*else {
                  println!("skipping runner {runner}");
              }*/
        }
        if !chosen {
            panic!("no runner chosen! cumulative: {cumulative}, random: {random}");
        }
    }

    // println!("podium: {podium:?}");
}

fn validate_args(
    probs: &Matrix<f64>,
    podium: &mut [usize],
    bitmap: &mut [bool],
    totals: &mut [f64],
) -> bool {
    assert!(
        !probs.is_empty(),
        "the probabilities matrix cannot be empty"
    );
    assert!(!podium.is_empty(), "the podium slice cannot be empty");
    assert!(
        podium.len() <= probs.cols(),
        "number of podium entries cannot exceed number of runners"
    );
    assert_eq!(
        probs.cols(),
        bitmap.len(),
        "a bitmap entry must exist for each runner"
    );
    assert_eq!(
        totals.len(),
        podium.len(),
        "a total must exist for each podium rank"
    );
    assert_eq!(
        probs.rows(),
        podium.len(),
        "a probability row must exist for each podium rank"
    );
    for p in probs.flatten() {
        assert!(
            (0.0..=1.0).contains(p),
            "probabilities out of range: {probs}"
        );
    }
    true
}

#[inline(always)]
fn random_f64(rand: &mut impl Rand) -> f64 {
    rand.next_u64() as f64 / u64::MAX as f64
}
