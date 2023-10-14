use tinyrand::Rand;

pub fn run_once(probs: &[f64], podium: &mut[usize], bitmap: &mut [bool], rand: &mut impl Rand) {
    debug_assert_eq!(probs.len(), bitmap.len());
    debug_assert!(podium.len() > 0);
    debug_assert!(podium.len() <= probs.len());
    debug_assert!(validate_probs(probs));

    let runners = probs.len();
    let mut prob_sum = 1.0;
    reset_bitmap(bitmap);
    println!("podium.len: {}", podium.len());
    for rank in 0..podium.len() {
        let mut cumulative = 0.0;
        let random = random_f64(rand) * prob_sum;
        println!("random={random:.3}, cumulative={cumulative}");
        for runner in 0..runners {
            if bitmap[runner] {
                let prob = probs[runner];
                cumulative += prob;
                println!("probs[{runner}]={prob:.3}, cumulative={cumulative:.3}");
                if cumulative >= random {
                    println!("chosen runner {runner} for rank {rank}");
                    podium[rank] = runner;
                    bitmap[runner] = false;
                    prob_sum -= prob;
                    break;
                }
            }
        }
    }

    println!("podium: {podium:?}");
}

fn validate_probs(probs: &[f64]) -> bool {
    for &p in probs {
        debug_assert!(p >= 0.0, "invalid probs {probs:?}");
        debug_assert!(p <= 1.0, "invalid probs {probs:?}");
    }
    true
}

fn reset_bitmap(bitmap: &mut [bool]) {
    for b in bitmap {
        *b = true;
    }
}

#[inline]
fn random_f64(rand: &mut impl Rand) -> f64 {
    rand.next_u64() as f64 / u64::MAX as f64
}