use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::market::{Market, Overround, OverroundMethod};
use crate::model::cf::Coefficients;
use crate::model::fit::{FitOptions, PlaceFitOutcome};
use crate::{mc, selection};
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
use tracing::debug;

pub mod cf;
pub mod error;
pub mod fit;

// pub const DEFAULT_OVERROUND_METHOD: OverroundMethod = OverroundMethod::Multiplicative;
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Top4Markets {
    pub markets: [Market; PODIUM],
}
impl Top4Markets {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
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
    // pub overround_method: OverroundMethod
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
    pub fn generate_top_4(&self, overrounds: &[Overround]) -> Result<Top4Markets, anyhow::Error> {
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
            Market::frame(&overround.method, probs, overround.value)
        }).collect();
        let markets: [Market; 4] = markets.try_into().unwrap();
        Ok(Top4Markets {
            markets
        })
    }
}
