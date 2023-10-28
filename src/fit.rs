use std::ops::{Deref, Range, RangeInclusive};
use std::time::Duration;

use tokio::time::Instant;

use mc::MonteCarloEngine;

use crate::mc;
use crate::capture::Capture;
use crate::linear::Matrix;
use crate::market::{Market, MarketPrice, OverroundMethod};
use crate::mc::DilatedProbs;
use crate::probs::SliceExt;
use crate::selection::{Rank, Runner, Selections};

const FITTED_PRICE_RANGES: [Range<f64>; 4] = [1.0..50.0, 1.0..15.0, 1.0..10.0, 1.0..5.0];
const MAX_INDIVIDUAL_STEPS: u64 = 100;

pub struct FitOptions {
    pub mc_iterations: u64,
    pub individual_target_msre: f64,
}

pub struct PlaceFitOutcome {
    pub stats: OptimiserStats,
    pub fitted_probs: Matrix<f64>,
}

pub fn fit_place(options: FitOptions, win_market: &Market, place_market: &Market, dilatives: &[f64], place_rank: usize) -> PlaceFitOutcome {
    let podium_places = dilatives.len();
    let num_runners = win_market.probs.len();
    let dilated_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&win_market.probs))
        .with_dilatives(Capture::Borrowed(dilatives))
        .into();

    let mut scenarios = Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            scenarios[(rank, runner)] = vec![Runner::index(runner).top(Rank::index(rank))].into();
        }
    }

    //TODO fit overrounds separately
    // let overrounds = match place_rank {
    //     1 => {
    //         let overround_step = win_overround - place_overround;
    //         vec![
    //             win_overround,
    //             place_overround,
    //             place_overround - overround_step,
    //             place_overround - 2.0 * overround_step,
    //         ]
    //     },
    //     2 => {
    //         let overround_step = (win_overround - place_overround) / 2.0;
    //         vec![
    //             win_overround,
    //             win_overround - overround_step,
    //             place_overround,
    //             place_overround - overround_step,
    //         ]
    //     },
    //     _ => unimplemented!("unsupported place rank {place_rank}")
    // };
    // let overrounds = vec![
    //     win_market.overround.value,
    //     place_market.overround.value
    // ];

    let outcome = fit_individual(
        &scenarios,
        &dilated_probs,
        options.mc_iterations,
        options.individual_target_msre,
        place_rank,
        1..=3,
        place_market.overround.value,
        &place_market.overround.method,
        &place_market.prices,
    );
    PlaceFitOutcome {
        stats: outcome.stats,
        fitted_probs: outcome.optimal_probs,
    }
}

pub fn compute_msre<P: MarketPrice>(
    sample_prices: &[f64],
    fitted_prices: &[P],
    price_range: &Range<f64>,
) -> f64 {
    let mut sq_rel_error = 0.0;
    let mut counted = 0;
    for (runner, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[runner].decimal();
        if fitted_price.is_finite() && price_range.contains(sample_price) {
            counted += 1;
            let relative_error = (sample_price - fitted_price) / sample_price;
            sq_rel_error += relative_error.powi(2);
        }
    }
    sq_rel_error / counted as f64
}

#[derive(Debug)]
pub struct OptimiserStats {
    pub optimal_msre: f64,
    pub steps: u64,
    pub time: Duration
}

#[derive(Debug)]
pub struct IndividualFitOutcome {
    pub stats: OptimiserStats,
    pub optimal_probs: Matrix<f64>,
}

fn fit_individual(
    scenarios: &Matrix<Selections>,
    dilated_probs: &Matrix<f64>,
    mc_iterations: u64,
    target_msre: f64,
    rank: usize,
    adj_ranks: RangeInclusive<usize>,
    overround: f64,
    overround_method: &OverroundMethod,
    sample_prices: &[f64],
) -> IndividualFitOutcome {
    let start_time = Instant::now();
    let podium_places = dilated_probs.rows();
    let num_runners = dilated_probs.cols();
    let mut engine = MonteCarloEngine::default()
        .with_iterations(mc_iterations)
        .with_probs(Capture::Borrowed(dilated_probs));

    let mut optimal_msre = f64::MAX;
    let mut optimal_probs = Matrix::empty();
    let mut step = 0;
    while step < MAX_INDIVIDUAL_STEPS {
        step += 1;
        println!("INDIVIDUAL FITTING step {step}");
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
        let fitted_probs: Vec<_> = counts.row_slice(rank).iter().map(|&count| count as f64 / engine.iterations() as f64).collect();
        let market = Market::frame(overround_method, fitted_probs, overround);

        // let mut derived_prices = Matrix::allocate(podium_places, num_runners);
        // for runner in 0..num_runners {
        //     for rank in 0..podium_places {
        //         let probability = counts[(rank, runner)] as f64 / engine.iterations() as f64;
        //         let fair_price = 1.0 / probability;
        //         let market_price = overround::apply_with_cap(fair_price, overround);
        //         derived_prices[(rank, runner)] = market_price;
        //     }
        // }
        // let fitted_prices = derived_prices.row_slice(rank);
        println!("fitted prices:  {:?}", market.prices);
        println!("sample prices: {sample_prices:?}");
        let msre = compute_msre(sample_prices, &market.prices, &FITTED_PRICE_RANGES[rank]);
        println!("msre: {msre}, rmsre: {}", msre.sqrt());

        let mut current_probs = engine.probs().unwrap().deref().clone();
        if msre < optimal_msre {
            optimal_msre = msre;
            optimal_probs = current_probs.clone();
        } else if msre < target_msre {
            break;
        }

        // let mut adjustments = vec![0.0; place_prices.len()];
        for (runner, sample_price) in sample_prices.iter().enumerate() {
            if sample_price.is_finite() {
                let fitted_price = market.prices[runner];
                let adj = fitted_price / sample_price;
                // adjustments[runner] = adj;
                for rank in adj_ranks.clone() {
                    scale_prob_capped(&mut current_probs[(rank, runner)], adj);
                }
            };
        }
        for rank in adj_ranks.clone() {
            current_probs.row_slice_mut(rank).normalise(1.0);
        }
        // println!("adjustments: {adjustments:?}");
        println!("adjusted probs: {:?}", current_probs.row_slice(rank));
        engine.reset_rand();
        engine.set_probs(current_probs.into());
    }

    let time = start_time.elapsed();
    IndividualFitOutcome {
        stats: OptimiserStats {
            optimal_msre,
            steps: step,
            time
        },
        optimal_probs,
    }
}

#[inline(always)]
fn scale_prob_capped(prob: &mut f64, adj: f64) {
    let scaled = f64::max(0.0, f64::min(*prob * adj, 1.0));
    *prob = scaled
}
