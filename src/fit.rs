use std::ops::{Deref, Range, RangeInclusive};
use std::time::Duration;
use strum::EnumCount;

use tokio::time::Instant;
use tracing::trace;

use mc::MonteCarloEngine;

use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::market::{Market, MarketPrice, OverroundMethod};
use crate::mc::DilatedProbs;
use crate::probs::SliceExt;
use crate::selection::{Rank, Selections};
use crate::{mc, selection};
use crate::data::Factor;

// const FITTED_PRICE_RANGES: [Range<f64>; 4] = [1.0..50.0, 1.0..15.0, 1.0..10.0, 1.0..5.0];
const FITTED_PRICE_RANGES: [Range<f64>; 4] = [1.0..1001.0, 1.0..1001.0, 1.0..1001.0, 1.0..1001.0];
const MAX_INDIVIDUAL_STEPS: u64 = 100;

pub struct FitOptions {
    pub mc_iterations: u64,
    pub individual_target_msre: f64,
}

pub struct AllFitOutcome {
    pub stats: Vec<OptimiserStats>,
    pub fitted_probs: Matrix<f64>,
}

pub fn fit_all(options: FitOptions, markets: &[Market], dilatives: &[f64]) -> AllFitOutcome {
    let podium_places = dilatives.len();
    let num_runners = markets[0].probs.len();
    let mut weighted_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&markets[0].probs))
        .with_dilatives(Capture::Borrowed(dilatives))
        .into();

    let scenarios = selection::top_n_matrix(podium_places, num_runners);

    let outcomes: Vec<_> = (1..podium_places)
        .map(|rank| {
            let market = &markets[rank];
            let outcome = fit_individual(
                &scenarios,
                &weighted_probs,
                options.mc_iterations,
                options.individual_target_msre,
                rank,
                rank..=rank,
                market.overround.value,
                &market.overround.method,
                &market.prices,
            );
            weighted_probs = outcome.optimal_probs.clone();
            outcome
        })
        .collect();

    // let fitted_probs = outcomes[podium_places - 2].optimal_probs.clone();
    // let stats = outcomes.into_iter().map(|outcome| outcome.stats).collect();
    AllFitOutcome {
        stats: outcomes.into_iter().map(|outcome| outcome.stats).collect(),
        fitted_probs: weighted_probs,
    }
}

pub struct PlaceFitOutcome {
    pub stats: OptimiserStats,
    pub fitted_probs: Matrix<f64>,
}

pub fn fit_place(
    options: FitOptions,
    win_market: &Market,
    place_market: &Market,
    dilatives: &[f64],
    place_rank: usize,
) -> PlaceFitOutcome {
    let podium_places = dilatives.len();
    let num_runners = win_market.probs.len();
    let active_runners = win_market.probs.iter().filter(|&&prob| prob != 0.).count() as f64;
    let mut weighted_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&win_market.probs))
        .with_dilatives(Capture::Borrowed(dilatives))
        .into();

    struct Coefficients {
        win: f64,
        win_squared: f64,
        win_cubed: f64,
        num_runners: f64,
        num_runners_squared: f64,
        num_runners_cubed: f64,
        stdev: f64,
        stdev_squared: f64,
        stdev_cubed: f64,
    }
    // let cf_1 = Coefficients {
    //     win: 1.594e+00,
    //     win_squared: -3.831e+00,
    //     win_cubed: 3.821e+00,
    //     num_runners: 3.829e-04,
    //     stdev: -1.048e+00,
    //     stdev_squared: 1.404e+01,
    //     stdev_cubed: -4.393e+01,
    // };
    // let cf_1 = Coefficients {
    //     win: 1.342647,
    //     win_squared: -2.567398,
    //     win_cubed: 2.221628,
    //     num_runners: 0.0,
    //     stdev: 0.040785,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_1 = Coefficients {
    //     win: 1.42531,
    //     win_squared: -3.03113,
    //     win_cubed: 2.92645,
    //     num_runners: 0.0,
    //     num_runners_squared: 0.0,
    //     num_runners_cubed: 0.0,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    let cf_1 = Coefficients {
        win: 1.490e+00,
        win_squared: -3.358e+00,
        win_cubed: 3.394e+00,
        num_runners: -2.079e-04,
        num_runners_squared: 0.0,
        num_runners_cubed: 0.0,
        stdev: 0.0,
        stdev_squared: 0.0,
        stdev_cubed: 0.0,
    };
    // let cf_1 = Coefficients {
    //     win: 1.529e+00,
    //     win_squared: -3.706e+00,
    //     win_cubed: 3.645e+00,
    //     num_runners: 3.723e-03,
    //     num_runners_squared: -5.394e-04,
    //     num_runners_cubed: 1.864e-05,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_2 = Coefficients {
    //     win: 1.377e+00,
    //     win_squared: -3.988e+00,
    //     win_cubed: 4.381e+00,
    //     num_runners: 1.081e-03,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_2 = Coefficients {
    //     win: 1.7140,
    //     win_squared: -5.8031,
    //     win_cubed: 7.0702,
    //     num_runners: 0.0,
    //     num_runners_squared: 0.0,
    //     num_runners_cubed: 0.0,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_2 = Coefficients {
    //     win: 1.406e+00,
    //     win_squared: -4.080e+00,
    //     win_cubed: 4.506e+00,
    //     num_runners: 9.996e-04,
    //     num_runners_squared: 0.0,
    //     num_runners_cubed: 0.0,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    let cf_2 = Coefficients {
        win: 1.273e+00,
        win_squared: -3.425e+00,
        win_cubed: 3.621e+00,
        num_runners: 8.103e-03,
        num_runners_squared: -8.194e-04,
        num_runners_cubed: 2.372e-05,
        stdev: 0.0,
        stdev_squared: 0.0,
        stdev_cubed: 0.0,
    };
    // let cf_2 = Coefficients {
    //     win: 1.258e+00,
    //     win_squared: -3.455e+00,
    //     win_cubed: 3.630e+00,
    //     num_runners: 1.355e-02,
    //     num_runners_squared: -1.529e-03,
    //     num_runners_cubed: 4.703e-05,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_3 = Coefficients {
    //     win: 1.3689085,
    //     win_squared: -4.5581003,
    //     win_cubed: 4.9730957,
    //     num_runners: -0.0004909,
    //     stdev: 0.2647938,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 8.0392854,
    // };
    // let cf_3 = Coefficients {
    //     win: 1.89903,
    //     win_squared: -7.58522,
    //     win_cubed: 9.70426,
    //     num_runners: 0.0,
    //     num_runners_squared: 0.0,
    //     num_runners_cubed: 0.0,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    // let cf_3 = Coefficients {
    //     win: 1.448e+00,
    //     win_squared: -4.982e+00,
    //     win_cubed: 5.813e+00,
    //     num_runners: 1.459e-03,
    //     num_runners_squared: 0.0,
    //     num_runners_cubed: 0.0,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };
    let cf_3 = Coefficients {
        win: 1.254e+00,
        win_squared: -4.023e+00,
        win_cubed: 4.514e+00,
        num_runners: 1.145e-02,
        num_runners_squared: -1.139e-03,
        num_runners_cubed: 3.254e-05,
        stdev: 0.0,
        stdev_squared: 0.0,
        stdev_cubed: 0.0,
    };
    // let cf_3 = Coefficients {
    //     win: 1.205e+00,
    //     win_squared: -3.889e+00 ,
    //     win_cubed: 4.346e+00,
    //     num_runners: 1.840e-02,
    //     num_runners_squared: -2.037e-03,
    //     num_runners_cubed: 6.192e-05,
    //     stdev: 0.0,
    //     stdev_squared: 0.0,
    //     stdev_cubed: 0.0,
    // };

    fn linear_sum(cf: &Coefficients, win_prob: f64, active_runners: f64, stdev: f64) -> f64 {
        win_prob * cf.win
            + win_prob.powi(2) * cf.win_squared
            + win_prob.powi(3) * cf.win_cubed
            + active_runners * cf.num_runners
            + active_runners.powi(2) * cf.num_runners_squared
            + active_runners.powi(3) * cf.num_runners_cubed
            + stdev * cf.stdev
            + stdev.powi(2) * cf.stdev_squared
            + stdev.powi(3) * cf.stdev_cubed
    }

    let stdev = win_market.probs.stdev();
    let places_paying = place_rank as f64 + 1.;
    let mut input = [0.; Factor::COUNT];
    for runner in 0..num_runners {
        let win_prob = win_market.probs[runner];
        if win_prob != 0.0 {
            input[Factor::RunnerIndex.ordinal()] = runner as f64;
            input[Factor::ActiveRunners.ordinal()] = active_runners;
            input[Factor::PlacesPaying.ordinal()] = places_paying;
            input[Factor::Stdev.ordinal()] = stdev;
            input[Factor::Weight0.ordinal()] = win_prob;

            //TODO
            weighted_probs[(1, runner)] = linear_sum(&cf_1, win_prob, active_runners, stdev);
            weighted_probs[(2, runner)] = linear_sum(&cf_2, win_prob, active_runners, stdev);
            weighted_probs[(3, runner)] = linear_sum(&cf_3, win_prob, active_runners, stdev);
        }
    }
    for rank in 1..podium_places {
        weighted_probs.row_slice_mut(rank).normalise(1.0);
    }

    let scenarios = selection::top_n_matrix(podium_places, num_runners);
    let outcome = fit_individual(
        &scenarios,
        &weighted_probs,
        options.mc_iterations,
        options.individual_target_msre,
        place_rank,
        // place_rank..=place_rank,//1..=3, //todo
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
    pub time: Duration,
}

#[derive(Debug)]
pub struct IndividualFitOutcome {
    pub stats: OptimiserStats,
    pub optimal_probs: Matrix<f64>,
}

fn fit_individual(
    scenarios: &Matrix<Selections>,
    weighted_probs: &Matrix<f64>,
    mc_iterations: u64,
    target_msre: f64,
    rank: usize,
    adj_ranks: RangeInclusive<usize>,
    overround: f64,
    overround_method: &OverroundMethod,
    sample_prices: &[f64],
) -> IndividualFitOutcome {
    let start_time = Instant::now();
    let podium_places = weighted_probs.rows();
    let num_runners = weighted_probs.cols();
    let mut engine = MonteCarloEngine::default()
        .with_iterations(mc_iterations)
        .with_probs(Capture::Borrowed(weighted_probs));

    let mut optimal_msre = f64::MAX;
    let mut optimal_probs = Matrix::empty();
    let mut step = 0;
    while step < MAX_INDIVIDUAL_STEPS {
        step += 1;
        trace!(
            "individual fitting step {step} for rank {}",
            Rank::index(rank)
        );
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
        let fitted_probs: Vec<_> = counts
            .row_slice(rank)
            .iter()
            .map(|&count| count as f64 / engine.iterations() as f64)
            .collect();
        let market = Market::frame(overround_method, fitted_probs, overround);
        trace!("fitted prices:  {:?}", market.prices);
        trace!("sample prices: {sample_prices:?}");
        let msre = compute_msre(sample_prices, &market.prices, &FITTED_PRICE_RANGES[rank]);
        trace!("msre: {msre}, rmsre: {}", msre.sqrt());

        let mut current_probs = engine.probs().unwrap().deref().clone();
        if msre < optimal_msre {
            optimal_msre = msre;
            optimal_probs = current_probs.clone();
        } else if msre < target_msre {
            break;
        }

        for (runner, sample_price) in sample_prices.iter().enumerate() {
            if sample_price.is_finite() {
                let fitted_price = market.prices[runner];
                let adj = fitted_price / sample_price;
                for rank in adj_ranks.clone() {
                    scale_prob_capped(&mut current_probs[(rank, runner)], adj);
                }
            };
        }
        for rank in adj_ranks.clone() {
            current_probs.row_slice_mut(rank).normalise(1.0);
        }
        trace!("adjusted probs: {:?}", current_probs.row_slice(rank));
        engine.reset_rand();
        engine.set_probs(current_probs.into());
    }

    let time = start_time.elapsed();
    IndividualFitOutcome {
        stats: OptimiserStats {
            optimal_msre,
            steps: step,
            time,
        },
        optimal_probs,
    }
}

#[inline(always)]
fn scale_prob_capped(prob: &mut f64, adj: f64) {
    let scaled = f64::max(0.0, f64::min(*prob * adj, 1.0));
    *prob = scaled
}
