use crate::selection::Selection;
use tinyrand::Rand;
use crate::probs::Fraction;

pub fn run_many(
    iters: u64,
    selections: &[Selection],
    probs: &[f64],
    podium: &mut [usize],
    bitmap: &mut [bool],
    rand: &mut impl Rand,
) -> Fraction {
    let mut matching_iters = 0;
    for _ in 0..iters {
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
    Fraction { numerator: matching_iters, denominator: iters }
}

#[inline]
pub fn run_once(probs: &[f64], podium: &mut [usize], bitmap: &mut [bool], rand: &mut impl Rand) {
    debug_assert_eq!(probs.len(), bitmap.len());
    debug_assert!(podium.len() > 0);
    debug_assert!(podium.len() <= probs.len());
    debug_assert!(validate_probs(probs));

    let runners = probs.len();
    let mut prob_sum = 1.0;
    reset_bitmap(bitmap);
    // println!("podium.len: {}", podium.len());
    for rank in 0..podium.len() {
        let mut cumulative = 0.0;
        let random = random_f64(rand) * prob_sum;
        // println!("random={random:.3}, prob_sum={prob_sum}");
        for runner in 0..runners {
            if bitmap[runner] {
                let prob = probs[runner];
                cumulative += prob;
                // println!("probs[{runner}]={prob:.3}, cumulative={cumulative:.3}");
                if cumulative >= random {
                    // println!("chosen runner {runner} for rank {rank}");
                    podium[rank] = runner;
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

fn validate_probs(probs: &[f64]) -> bool {
    for &p in probs {
        debug_assert!(p >= 0.0, "invalid probs {probs:?}");
        debug_assert!(p <= 1.0, "invalid probs {probs:?}");
    }
    true
}

#[inline]
fn reset_bitmap(bitmap: &mut [bool]) {
    for b in bitmap {
        *b = true;
    }
}

#[inline]
fn random_f64(rand: &mut impl Rand) -> f64 {
    rand.next_u64() as f64 / u64::MAX as f64
}
