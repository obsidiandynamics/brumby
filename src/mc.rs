//! The core of the Monte Carlo simulator.

use std::ops::DerefMut;
use crate::probs::Fraction;
use crate::selection::Selection;
use tinyrand::{Rand, StdRand};
use crate::capture::{CaptureMut, Capture};

pub struct MonteCarloEngine<'a, R: Rand> {
    iterations: u64,
    probabilities: Capture<'a, Vec<f64>, [f64]>,
    podium: CaptureMut<'a, Vec<usize>, [usize]>,
    bitmap: CaptureMut<'a, Vec<bool>, [bool]>,
    rand: CaptureMut<'a, R, R>,
}
impl<'a, R: Rand> MonteCarloEngine<'a, R> {
    pub fn with_iterations(mut self, iterations: u64) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_probabilities(mut self, probabilities: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.probabilities = probabilities;
        self
    }

    pub fn with_podium_places(self, places: usize) -> Self {
        self.with_podium(CaptureMut::Owned(vec![usize::MAX; places]))
    }

    pub fn with_podium(mut self, podium: CaptureMut<'a, Vec<usize>, [usize]>) -> Self {
        self.podium = podium;
        self
    }

    pub fn with_bitmap(mut self, bitmap: CaptureMut<'a, Vec<bool>, [bool]>) -> Self {
        self.bitmap = bitmap;
        self
    }

    pub fn with_rand(mut self, rand: CaptureMut<'a, R, R>) -> Self {
        self.rand = rand;
        self
    }

    pub fn simulate(&mut self, selections: &[Selection]) -> Fraction {
        if self.bitmap.is_empty() {
            self.bitmap = CaptureMut::Owned(vec![true; self.probabilities.len()]);
        }

        run_many(
            self.iterations,
            selections,
            &self.probabilities,
            &mut self.podium,
            &mut self.bitmap,
           self.rand.deref_mut(),
        )
    }
}

impl Default for MonteCarloEngine<'_, StdRand> {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            probabilities: Capture::Owned(vec![]),
            podium: CaptureMut::Owned(vec![]),
            bitmap: CaptureMut::Owned(vec![]),
            rand: CaptureMut::Owned(StdRand::default()),
        }
    }
}

pub fn run_many(
    iterations: u64,
    selections: &[Selection],
    probabilities: &[f64],
    podium: &mut [usize],
    bitmap: &mut [bool],
    rand: &mut impl Rand,
) -> Fraction {
    assert!(validate_params(probabilities, podium, bitmap));

    let mut matching_iters = 0;
    for _ in 0..iterations {
        run_once(probabilities, podium, bitmap, rand);
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
pub fn run_once(probabilities: &[f64], podium: &mut [usize], bitmap: &mut [bool], rand: &mut impl Rand) {
    debug_assert!(validate_params(probabilities, podium, bitmap));

    let runners = probabilities.len();
    let mut prob_sum = 1.0;
    reset_bitmap(bitmap);
    // println!("podium.len: {}", podium.len());
    for ranked_runner in podium.iter_mut() {
        let mut cumulative = 0.0;
        let random = random_f64(rand) * prob_sum;
        // println!("random={random:.3}, prob_sum={prob_sum}");
        for runner in 0..runners {
            if bitmap[runner] {
                let prob = probabilities[runner];
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

fn validate_params(probabilities: &[f64], podium: &mut [usize], bitmap: &mut [bool]) -> bool {
    assert!(!probabilities.is_empty());
    assert_eq!(probabilities.len(), bitmap.len());
    assert!(!podium.is_empty());
    assert!(podium.len() <= probabilities.len());
    for &p in probabilities {
        assert!(p >= 0.0, "invalid probabilities {probabilities:?}");
        assert!(p <= 1.0, "invalid probabilities {probabilities:?}");
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
