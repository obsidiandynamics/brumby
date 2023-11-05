use crate::opt;
use crate::opt::DescentConfig;
use crate::probs::SliceExt;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

const MIN_PRICE: f64 = 1.04;
const MAX_PRICE: f64 = 10001.0;

pub trait MarketPrice {
    fn decimal(&self) -> f64;
}

impl MarketPrice for f64 {
    fn decimal(&self) -> f64 {
        *self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Overround {
    pub method: OverroundMethod,
    pub value: f64,
}
impl Overround {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        const MIN_OVERROUND: f64 = 1.;
        if self.value < MIN_OVERROUND {
            bail!("overround cannot be less than {MIN_OVERROUND}");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverroundMethod {
    Multiplicative,
    Power,
    OddsRatio,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Market {
    pub probs: Vec<f64>,
    pub prices: Vec<f64>,
    pub overround: Overround,
}
impl Market {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        const VALID_PROB_RANGE: RangeInclusive<f64> = 0.0..=1.;
        if self
            .probs
            .iter()
            .any(|prob| !VALID_PROB_RANGE.contains(prob))
        {
            bail!("probabilities must lie in the range: {VALID_PROB_RANGE:?}");
        }
        const MIN_PRICE: f64 = 1.;
        if self.prices.iter().any(|&price| price < 1.) {
            bail!("prices cannot be lower than {MIN_PRICE}");
        }
        if self.prices.iter().any(|&price| price.is_nan()) {
            bail!("prices cannot be NaN");
        }
        const MIN_PROBS: usize = 2;
        if self.probs.len() < MIN_PROBS {
            bail!("the number of provided probabilities cannot be fewer than {MIN_PROBS}");
        }
        if self.probs.len() != self.prices.len() {
            bail!("exactly one probability must be provided for each price");
        }
        if self
            .probs
            .iter()
            .zip(self.prices.iter())
            .any(|(&prob, &price)| {
                prob == 0. && price.is_finite() || prob != 0. && price.is_infinite()
            })
        {
            bail!("a zero probability must be accompanied by an infinite price and vice versa");
        }
        self.overround.validate()?;
        Ok(())
    }

    pub fn fit(method: &OverroundMethod, prices: Vec<f64>, fair_sum: f64) -> Self {
        match method {
            OverroundMethod::Multiplicative => Self::fit_multiplicative(prices, fair_sum),
            OverroundMethod::Power => Self::fit_power(prices, fair_sum),
            OverroundMethod::OddsRatio => Self::fit_odds_ratio(prices, fair_sum),
        }
    }

    pub fn frame(overround: &Overround, probs: Vec<f64>) -> Self {
        match overround.method {
            OverroundMethod::Multiplicative => Self::frame_multiplicative(probs, overround.value),
            OverroundMethod::Power => Self::frame_power(probs, overround.value),
            OverroundMethod::OddsRatio => Self::frame_odds_ratio(probs, overround.value)
        }
    }

    fn fit_multiplicative(prices: Vec<f64>, fair_sum: f64) -> Self {
        let mut probs: Vec<_> = prices.invert().collect();
        let overround = probs.normalise(fair_sum) / fair_sum;
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround,
            },
        }
    }

    fn fit_power(prices: Vec<f64>, fair_sum: f64) -> Market {
        let overround = prices.invert().sum::<f64>() / fair_sum;
        let est_rtp = 1.0 / overround;
        let initial_k = 1.0 + f64::ln(est_rtp) / f64::ln(prices.len() as f64);
        // println!("fit_power: initial_k: {initial_k}");
        let outcome = opt::descent(
            DescentConfig {
                init_value: initial_k,
                step: -0.01,
                min_step: 0.0001,
                max_steps: 100_000,
                max_residual: 1e-9,
            },
            |exponent| {
                let mut sum = 0.0;
                for &price in &prices {
                    let scaled_price = (price * fair_sum).powf(exponent);
                    sum += 1.0 / scaled_price;
                }

                (sum - 1.0).powi(2)
            },
        );
        // println!("fit_power: outcome: {outcome:?}");

        let probs = prices
            .iter()
            .map(|price| {
                let scaled_price = (price * fair_sum).powf(outcome.optimal_value);
                fair_sum / scaled_price
            })
            .collect();

        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Power,
                value: overround,
            },
        }
    }

    fn fit_odds_ratio(prices: Vec<f64>, fair_sum: f64) -> Market {
        let overround = prices.invert().sum::<f64>() / fair_sum;
        let initial_d = overround;
        let outcome = opt::descent(
            DescentConfig {
                init_value: initial_d,
                step: 0.1,
                min_step: 0.0001,
                max_steps: 100_000,
                max_residual: 1e-9,
            },
            |d| {
                let mut sum = 0.0;
                for &price in &prices {
                    let uncapped_scaled_price = 1.0 + (price - 1.0) / d;
                    let capped_scaled_price =
                        cap(uncapped_scaled_price, MIN_PRICE, MAX_PRICE);
                    sum += 1.0 / capped_scaled_price;
                }

                (sum - fair_sum).powi(2)
            },
        );

        let probs = prices
            .iter()
            .map(|price| {
                let scaled_price = 1.0 + (price - 1.0) / outcome.optimal_value;
                1.0 / scaled_price
            })
            .collect();

        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::OddsRatio,
                value: overround,
            },
        }
    }

    fn frame_multiplicative(probs: Vec<f64>, overround: f64) -> Self {
        let prices: Vec<_> = probs
            .iter()
            .map(|prob| multiply_capped(1.0 / prob, overround))
            .collect();
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround,
            },
        }
    }

    fn frame_power(probs: Vec<f64>, overround: f64) -> Market {
        let rtp = 1.0 / overround;
        let fair_sum = probs.sum();
        let initial_k = 1.0 + f64::ln(rtp) / f64::ln(probs.len() as f64);
        let min_scaled_price = 1.0 + (MIN_PRICE - 1.0) / fair_sum;
        let max_scaled_price = 1.0 + (MAX_PRICE - 1.0) / fair_sum;
        let outcome = opt::descent(
            DescentConfig {
                init_value: initial_k,
                step: -0.01,
                min_step: 0.0001,
                max_steps: 100_000,
                max_residual: 1e-9,
            },
            |exponent| {
                let mut sum = 0.0;
                for &prob in &probs {
                    let uncapped_scaled_price = (fair_sum / prob).powf(exponent);
                    let capped_scaled_price =
                        cap(uncapped_scaled_price, min_scaled_price, max_scaled_price);
                    sum += 1.0 / capped_scaled_price;
                }

                (sum - overround).powi(2)
            },
        );

        let prices = probs
            .iter()
            .map(|prob| {
                let uncapped_price = (fair_sum / prob).powf(outcome.optimal_value) / fair_sum;
                if uncapped_price.is_finite() {
                    cap(uncapped_price, MIN_PRICE, MAX_PRICE)
                } else {
                    uncapped_price
                }
            })
            .collect();

        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Power,
                value: overround,
            },
        }
    }

    fn frame_odds_ratio(probs: Vec<f64>, overround: f64) -> Market {
        let fair_sum = probs.sum();
        let overround_sum = fair_sum * overround;
        let initial_d = overround;
        let outcome = opt::descent(
            DescentConfig {
                init_value: initial_d,
                step: 0.1,
                min_step: 0.0001,
                max_steps: 100_000,
                max_residual: 1e-9,
            },
            |d| {
                let mut sum = 0.0;
                for &prob in &probs {
                    let price = 1.0 / prob;
                    let uncapped_scaled_price = 1.0 + (price - 1.0) / d;
                    let capped_scaled_price =
                        cap(uncapped_scaled_price, MIN_PRICE, MAX_PRICE);
                    sum += 1.0 / capped_scaled_price;
                }

                (sum - overround_sum).powi(2)
            },
        );

        let prices = probs
            .iter()
            .map(|prob| {
                let price = 1.0 / prob;
                let uncapped_price = 1.0 + (price - 1.0) / outcome.optimal_value;
                if uncapped_price.is_finite() {
                    cap(uncapped_price, MIN_PRICE, MAX_PRICE)
                } else {
                    uncapped_price
                }
            })
            .collect();

        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::OddsRatio,
                value: overround,
            },
        }
    }
}

#[inline]
pub fn multiply_capped(fair_price: f64, overround: f64) -> f64 {
    let quotient = fair_price / overround;
    if quotient.is_finite() {
        cap(quotient, MIN_PRICE, MAX_PRICE)
    } else {
        quotient
    }
}

#[inline]
fn cap(value: f64, min: f64, max: f64) -> f64 {
    f64::min(f64::max(min, value), max)
}

#[cfg(test)]
mod tests;
