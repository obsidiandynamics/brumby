use std::ops::{Deref, Range, RangeInclusive};
use std::time::Duration;

use anyhow::bail;
use serde::{Deserialize, Serialize};
use strum::EnumCount;
use tokio::time::Instant;
use tracing::trace;

use mc::MonteCarloEngine;

use crate::{mc, model, selection};
use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::market::{Market, MarketPrice, Overround};
use crate::mc::DilatedProbs;
use crate::model::cf::{Coefficients, Factor};
use crate::probs::SliceExt;
use crate::selection::{Rank, Selections};

pub const FITTED_PRICE_RANGES: [Range<f64>; 4] = [1.0..1001.0, 1.0..1001.0, 1.0..1001.0, 1.0..1001.0];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitOptions {
    pub mc_iterations: u64,
    pub individual_target_msre: f64,
    pub max_individual_steps: u64,
}

impl FitOptions {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        const MIN_MC_ITERATIONS: u64 = 1_000;
        if self.mc_iterations < MIN_MC_ITERATIONS  {
            bail!("number of Monte Carlo iterations cannot be fewer than {MIN_MC_ITERATIONS}");
        }
        const MIN_TARGET_MSRE: f64 = f64::MIN_POSITIVE;
        if self.individual_target_msre < MIN_TARGET_MSRE {
            bail!("target MSRE cannot be less than {MIN_TARGET_MSRE}");
        }
        const MIN_MAX_INDIVIDUAL_STEPS: u64 = 10;
        if self.max_individual_steps < MIN_MAX_INDIVIDUAL_STEPS {
            bail!("maximum number of individual fitting steps cannot be fewer than {MIN_MAX_INDIVIDUAL_STEPS}");
        }
        Ok(())
    }

    /// Ultrafast presets when accuracy is unimportant (e.g., a demo).
    pub fn fast() -> Self {
        Self {
            mc_iterations: 1_000,
            individual_target_msre: 1e-3,
            max_individual_steps: 10,
        }
    }
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            mc_iterations: 100_000,
            individual_target_msre: 1e-6,
            max_individual_steps: 100,
        }
    }
}

pub struct AllFitOutcome {
    pub stats: Vec<OptimiserStats>,
    pub fitted_probs: Matrix<f64>,
}

pub fn fit_all(options: FitOptions, markets: &[Market]) -> Result<AllFitOutcome, anyhow::Error> {
    options.validate()?;
    for market in markets {
        market.validate()?;
    }
    let num_runners = markets[0].probs.len();
    let mut weighted_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&markets[0].probs))
        .with_podium_places(model::PODIUM)
        .into();

    let scenarios = selection::top_n_matrix(model::PODIUM, num_runners);

    let outcomes: Vec<_> = (1..model::PODIUM)
        .map(|rank| {
            let market = &markets[rank];
            let outcome = fit_individual(
                &scenarios,
                &weighted_probs,
                options.mc_iterations,
                options.individual_target_msre,
                options.max_individual_steps,
                rank,
                rank..=rank,
                &market.overround,
                &market.prices,
            );
            weighted_probs = outcome.optimal_probs.clone();
            outcome
        })
        .collect();

    Ok(AllFitOutcome {
        stats: outcomes.into_iter().map(|outcome| outcome.stats).collect(),
        fitted_probs: weighted_probs,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceFitOutcome {
    pub stats: OptimiserStats,
    pub fitted_probs: Matrix<f64>,
}

pub fn fit_place(
    coefficients: &Coefficients,
    options: &FitOptions,
    win_market: &Market,
    place_market: &Market,
    place_rank: usize,
) -> Result<PlaceFitOutcome, anyhow::Error> {
    coefficients.validate()?;
    options.validate()?;
    let num_runners = win_market.probs.len();
    let active_runners = win_market.probs.iter().filter(|&&prob| prob != 0.).count() as f64;
    let mut weighted_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&win_market.probs))
        .with_podium_places(model::PODIUM)
        .into();

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

            weighted_probs[(1, runner)] = cap_probability(coefficients.w1.predict(&input));
            weighted_probs[(2, runner)] = cap_probability(coefficients.w2.predict(&input));
            weighted_probs[(3, runner)] = cap_probability(coefficients.w3.predict(&input));
        }
    }
    for rank in 1..model::PODIUM {
        weighted_probs.row_slice_mut(rank).normalise(1.0);
    }

    let scenarios = selection::top_n_matrix(model::PODIUM, num_runners);
    let outcome = fit_individual(
        &scenarios,
        &weighted_probs,
        options.mc_iterations,
        options.individual_target_msre,
        options.max_individual_steps,
        place_rank,
        // place_rank..=place_rank, //todo
        1..=model::PODIUM - 1,
        &place_market.overround,
        &place_market.prices,
    );
    Ok(PlaceFitOutcome {
        stats: outcome.stats,
        fitted_probs: outcome.optimal_probs,
    })
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

#[derive(Debug, Clone, PartialEq)]
pub struct OptimiserStats {
    pub optimal_msre: f64,
    pub steps: u64,
    pub elapsed: Duration,
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
    max_individual_steps: u64,
    rank: usize,
    adj_ranks: RangeInclusive<usize>,
    overround: &Overround,
    sample_prices: &[f64],
) -> IndividualFitOutcome {
    let start_time = Instant::now();
    let podium_places = weighted_probs.rows();
    let runners = weighted_probs.cols();
    let mut engine = MonteCarloEngine::default()
        .with_iterations(mc_iterations)
        .with_probs(Capture::Borrowed(weighted_probs));

    let mut optimal_msre = f64::MAX;
    let mut optimal_probs = Matrix::empty();
    let mut step = 0;
    while step < max_individual_steps {
        step += 1;
        trace!(
            "individual fitting step {step} for rank {}",
            Rank::index(rank)
        );
        let mut counts = Matrix::allocate(podium_places, runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
        let fitted_probs: Vec<_> = counts
            .row_slice(rank)
            .iter()
            .map(|&count| count as f64 / engine.iterations() as f64)
            .collect();
        let market = Market::frame(overround, fitted_probs);
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

    let elapsed = start_time.elapsed();
    IndividualFitOutcome {
        stats: OptimiserStats {
            optimal_msre,
            steps: step,
            elapsed,
        },
        optimal_probs,
    }
    // //TODO remove
    // IndividualFitOutcome {
    //     stats: OptimiserStats {
    //         optimal_msre,
    //         steps: step,
    //         elapsed,
    //     },
    //     optimal_probs: weighted_probs.clone(),
    // }
}

#[inline(always)]
fn scale_prob_capped(prob: &mut f64, adj: f64) {
    *prob = cap_probability(*prob * adj)
}

/// Smallest permissible probability used for capping values produced by the linear model.
pub const PROBABILITY_EPSILON: f64 = 1e-6;

/// Caps a probability in the interval \[0 + epsilon, 1 - epsilon], where `epsilon` is the smallest
/// permissible probability, defined by [PROBABILITY_EPSILON].
#[inline(always)]
fn cap_probability(value: f64) -> f64 {
    f64::max(PROBABILITY_EPSILON, f64::min(value, 1.0 - PROBABILITY_EPSILON))
}
