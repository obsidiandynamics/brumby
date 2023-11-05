use std::ops::RangeInclusive;

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::market::{Market, Overround};
use crate::model::cf::Coefficients;
use crate::model::fit::{FitOptions, PlaceFitOutcome};
use crate::print::DerivedPrice;
use crate::selection::{validate_plausible_selections, Selection};
use crate::timed::Timed;
use crate::{market, mc, selection};

pub mod cf;
pub mod fit;

pub const PODIUM: usize = 4;
pub const LOWEST_PROBABILITY: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub struct WinPlace {
    pub win: Market,
    pub place: Market,
    pub places_paying: usize,
}
impl WinPlace {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        self.win.validate()?;
        self.place.validate()?;
        validate_correlated_markets([&self.win, &self.place].into_iter())?;
        const VALID_PLACES_PAYING: RangeInclusive<usize> = 2..=3;
        if !VALID_PLACES_PAYING.contains(&self.places_paying) {
            bail!("number of places paying must be in the range {VALID_PLACES_PAYING:?}");
        }
        Ok(())
    }

    pub fn extrapolate_overrounds(&self) -> Result<[Overround; PODIUM], anyhow::Error> {
        self.validate()?;
        let overround_step = (self.win.overround.value - self.place.overround.value)
            / (self.places_paying - 1) as f64;
        let overround_method = &self.win.overround.method;
        const MIN_OVERROUND: f64 = 1.01;
        let overrounds = match self.places_paying {
            2 => [
                self.win.overround.clone(),
                self.place.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(MIN_OVERROUND, self.place.overround.value - overround_step),
                },
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(
                        MIN_OVERROUND,
                        self.place.overround.value - 2. * overround_step,
                    ),
                },
            ],
            3 => [
                self.win.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(MIN_OVERROUND, self.win.overround.value - overround_step),
                },
                self.place.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(MIN_OVERROUND, self.place.overround.value - overround_step),
                },
            ],
            other => {
                bail!("unsupported number of places paying {other}");
            }
        };
        Ok(overrounds)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopN {
    pub markets: Vec<Market>,
}
impl TopN {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.markets.is_empty() {
            bail!("markets cannot be empty");
        }
        for market in &self.markets {
            market.validate()?;
        }
        validate_correlated_markets(self.markets.iter())?;
        Ok(())
    }

    pub fn overrounds(&self) -> Result<Vec<Overround>, anyhow::Error> {
        self.validate()?;
        let overrounds: Vec<_> = self
            .markets
            .iter()
            .map(|market| market.overround.clone())
            .collect();
        Ok(overrounds)
    }

    pub fn as_price_matrix(&self) -> Matrix<DerivedPrice> {
        let runners = self.markets[0].probs.len();
        let ranks = self.markets.len();
        let mut matrix = Matrix::allocate(ranks, runners);
        for (rank, market) in self.markets.iter().enumerate() {
            for runner in 0..runners {
                let (probability, price) = (market.probs[runner], market.prices[runner]);
                matrix[(rank, runner)] = DerivedPrice { probability, price };
            }
        }
        matrix
    }
}

fn validate_correlated_markets<'a, M>(mut markets: M) -> Result<(), anyhow::Error>
where
    M: Iterator<Item = &'a Market>,
{
    let first: &Market = markets
        .next()
        .ok_or(anyhow!("at least one market must be present"))?;
    for other in markets {
        let other = other;
        if first.probs.len() != other.probs.len() {
            bail!("the number of probabilities across correlated markets must match");
        }
        if first
            .probs
            .iter()
            .zip(other.probs.iter())
            .any(|(&first_prob, &other_prob)| {
                first_prob == 0. && other_prob != 0. || first_prob != 0. && other_prob == 0.
            })
        {
            bail!("if one probability is zero, all corresponding correlated probabilities must be zero");
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitterConfig {
    pub coefficients: Coefficients,
    pub fit_options: FitOptions,
}
impl FitterConfig {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        self.coefficients.validate()?;
        self.fit_options.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Primer {
    coefficients: Coefficients,
}
impl Primer {
    pub fn prime(
        &self,
        win: &Market,
        places_paying: usize,
        mc_trials: u64,
        overrounds: &[Overround],
    ) -> Result<Timed<PrimedModel>, anyhow::Error> {
        Timed::result(|| {
            win.validate()?;
            if overrounds.len() != PODIUM {
                bail!("exactly {PODIUM} overrounds must be specified");
            }
            let weighted_probs = fit::init_weighted_probs(&self.coefficients, win, places_paying - 1)?;
            let top_n = derive_prices(mc_trials, &weighted_probs, overrounds);
            Ok(PrimedModel {
                mc_trials,
                weighted_probs,
                top_n,
            })
        })
    }
}

impl TryFrom<Coefficients> for Primer {
    type Error = anyhow::Error;

    fn try_from(coefficients: Coefficients) -> Result<Self, Self::Error> {
        coefficients.validate()?;
        Ok(Self { coefficients })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimedModel {
    pub mc_trials: u64,
    pub weighted_probs: Matrix<f64>,
    pub top_n: TopN,
}

impl Model for PrimedModel {
    fn weighted_probs(&self) -> &Matrix<f64> {
        &self.weighted_probs
    }

    fn prices(&self) -> &TopN {
        &self.top_n
    }

    fn derive_multi(&self, selections: &[Selection]) -> Result<Timed<DerivedPrice>, anyhow::Error> {
        derive_multi(
            &self.weighted_probs,
            &self.top_n,
            selections,
            self.mc_trials,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fitter {
    config: FitterConfig,
}
impl Fitter {
    pub fn fit(
        &self,
        wp: &WinPlace,
        overrounds: &[Overround],
    ) -> Result<Timed<FittedModel>, anyhow::Error> {
        Timed::result(|| {
            wp.validate()?;
            if overrounds.len() != PODIUM {
                bail!("exactly {PODIUM} overrounds must be specified");
            }
            let active_runners = wp.win.prices.iter().filter(|&&price| price > 0.).count();
            if active_runners < PODIUM {
                bail!("at least {PODIUM} active runners required");
            }

            let weighted_probs = fit::init_weighted_probs(
                &self.config.coefficients,
                &wp.win,
                &wp.places_paying - 1,
            )?;
            let fit_outcome = fit::fit_place(
                &self.config.fit_options,
                &weighted_probs,
                &wp.place,
                wp.places_paying - 1,
            )?;
            debug!(
                "calibration complete: optimal MSRE: {}, RMSRE: {}, {} steps took: {:.3}s",
                fit_outcome.stats.optimal_msre,
                fit_outcome.stats.optimal_msre.sqrt(),
                fit_outcome.stats.steps,
                fit_outcome.stats.elapsed.as_millis() as f64 / 1_000.
            );
            let top_n = derive_prices(
                self.config.fit_options.mc_trials,
                &fit_outcome.fitted_probs,
                overrounds,
            );
            Ok(FittedModel {
                mc_trials: self.config.fit_options.mc_trials,
                fit_outcome,
                top_n,
            })
        })
    }
}

impl TryFrom<FitterConfig> for Fitter {
    type Error = anyhow::Error;

    fn try_from(config: FitterConfig) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self { config })
    }
}

pub trait Model {
    fn weighted_probs(&self) -> &Matrix<f64>;
    fn prices(&self) -> &TopN;
    fn derive_multi(&self, selections: &[Selection]) -> Result<Timed<DerivedPrice>, anyhow::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FittedModel {
    pub mc_trials: u64,
    pub fit_outcome: PlaceFitOutcome,
    pub top_n: TopN,
}

impl Model for FittedModel {
    fn weighted_probs(&self) -> &Matrix<f64> {
        &self.fit_outcome.fitted_probs
    }

    fn prices(&self) -> &TopN {
        &self.top_n
    }

    fn derive_multi(&self, selections: &[Selection]) -> Result<Timed<DerivedPrice>, anyhow::Error> {
        derive_multi(
            &self.fit_outcome.fitted_probs,
            &self.top_n,
            selections,
            self.mc_trials,
        )
    }
}

fn derive_prices(
    mc_trials: u64,
    weighted_probs: &Matrix<f64>,
    overrounds: &[Overround],
) -> TopN {
    let mut engine = mc::MonteCarloEngine::default()
        .with_trials(mc_trials)
        .with_probs(Capture::Borrowed(weighted_probs));

    let runners = weighted_probs.cols();
    let mut counts = Matrix::allocate(PODIUM, runners);
    let scenarios = selection::top_n_matrix(PODIUM, runners);
    engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

    let mut derived_probs = Matrix::allocate(PODIUM, runners);
    for runner in 0..runners {
        for rank in 0..PODIUM {
            let probability = counts[(rank, runner)] as f64 / engine.trials() as f64;
            derived_probs[(rank, runner)] = probability;
        }
    }
    let markets: Vec<_> = derived_probs
        .into_iter()
        .enumerate()
        .map(|(rank, probs)| {
            let overround = &overrounds[rank];
            let probs = probs.to_vec();
            Market::frame(overround, probs)
        })
        .collect();
    TopN { markets }
}

fn derive_multi(
    probs: &Matrix<f64>,
    top_n: &TopN,
    selections: &[Selection],
    mc_trials: u64,
) -> Result<Timed<DerivedPrice>, anyhow::Error> {
    Timed::result(|| {
        validate_plausible_selections(selections)?;
        let mut overround = 1.;
        let win_probs = &probs[0];
        for selection in selections {
            selection.validate(0..=PODIUM - 1, win_probs)?;
            let (runner, rank) = match selection {
                Selection::Span { runner, ranks } => (runner.as_index(), ranks.end().as_index()),
                Selection::Exact { runner, rank } => (runner.as_index(), rank.as_index()),
            };
            let market = &top_n.markets[rank];
            let prob = market.probs[runner];
            if prob == 0. {
                bail!("cannot price a runner with zero probability");
            }
            let price = market.prices[runner];

            overround *= 1. / prob / price;
        }
        let mut engine = mc::MonteCarloEngine::default()
            .with_trials(mc_trials)
            .with_probs(Capture::Borrowed(probs));
        let frac = engine.simulate(selections);
        let probability = f64::max(LOWEST_PROBABILITY, frac.quotient());
        let price = market::multiply_capped(1.0 / probability, overround);
        Ok(DerivedPrice { probability, price })
    })
}
