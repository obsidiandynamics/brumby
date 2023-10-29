use std::ops::RangeInclusive;

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{mc, selection};
use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::market::{Market, Overround};
use crate::model::cf::Coefficients;
use crate::model::fit::{FitOptions, PlaceFitOutcome};

pub mod cf;
pub mod error;
pub mod fit;

pub const PODIUM: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct WinPlaceMarkets {
    pub win: Market,
    pub place: Market,
    pub places_paying: usize,
}
impl WinPlaceMarkets {
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

    pub fn extrapolate_overrounds(&self) -> [Overround; PODIUM] {
        let overround_step = (self.win.overround.value - self.place.overround.value) / (self.places_paying - 1) as f64;
        let overround_method = &self.win.overround.method;
        const MIN_OVERROUND: f64 = 1.01;
        match self.places_paying {
            2 => [ self.win.overround.clone(),
                self.place.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(MIN_OVERROUND, self.place.overround.value - overround_step),
                },
                Overround {
                    method: overround_method.clone(),
                    value:  f64::max(MIN_OVERROUND, self.place.overround.value - 2. * overround_step),
                },
            ],
            3 => [ self.win.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value: f64::max(MIN_OVERROUND, self.win.overround.value - overround_step),
                },
                self.place.overround.clone(),
                Overround {
                    method: overround_method.clone(),
                    value:  f64::max(MIN_OVERROUND, self.place.overround.value - overround_step),
                },
            ],
            _ => unimplemented!()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopNMarkets {
    pub markets: Vec<Market>,
}
impl TopNMarkets {
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
pub struct Config {
    pub coefficients: Coefficients,
    pub fit_options: FitOptions,
}
impl Config {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        self.coefficients.validate()?;
        self.fit_options.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Calibrator {
    config: Config,
}
impl Calibrator {
    pub fn calibrate(&self, wp_markets: WinPlaceMarkets) -> Result<Model, anyhow::Error> {
        wp_markets.validate()?;

        let fit_outcome = fit::fit_place(
            &self.config.coefficients,
            &FitOptions::default(),
            &wp_markets.win,
            &wp_markets.place,
            wp_markets.places_paying - 1,
        )?;
        debug!(
            "calibration complete: optimal MSRE: {}, RMSRE: {}, {} steps took: {:.3}s",
            fit_outcome.stats.optimal_msre,
            fit_outcome.stats.optimal_msre.sqrt(),
            fit_outcome.stats.steps,
            fit_outcome.stats.time.as_millis() as f64 / 1_000.
        );
        Ok(Model {
            mc_iterations: self.config.fit_options.mc_iterations,
            fit_outcome,
        })
    }
}

impl TryFrom<Config> for Calibrator {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self { config })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    mc_iterations: u64,
    fit_outcome: PlaceFitOutcome,
}
impl Model {
    pub fn generate_top_4(&self, overrounds: &[Overround]) -> Result<TopNMarkets, anyhow::Error> {
        if overrounds.len() != PODIUM {
            bail!("exactly {PODIUM} overrounds must be specified");
        }

        let mut engine = mc::MonteCarloEngine::default()
            .with_iterations(self.mc_iterations)
            .with_probs(Capture::Borrowed(&self.fit_outcome.fitted_probs));

        let runners = self.fit_outcome.fitted_probs.cols();
        let mut counts = Matrix::allocate(PODIUM, runners);
        let scenarios = selection::top_n_matrix(PODIUM, runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

        let mut derived_probs = Matrix::allocate(PODIUM, runners);
        for runner in 0..runners {
            for rank in 0..PODIUM {
                let probability = counts[(rank, runner)] as f64 / engine.iterations() as f64;
                derived_probs[(rank, runner)] = probability;
            }
        }
        let markets: Vec<_> = derived_probs.into_iter().enumerate().map(|(rank, probs)| {
            let overround = &overrounds[rank];
            let probs = probs.to_vec();
            Market::frame(&overround, probs)
        }).collect();
        Ok(TopNMarkets {
            markets
        })
    }
}
