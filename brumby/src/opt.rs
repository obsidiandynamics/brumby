use std::ops::RangeInclusive;
use anyhow::bail;

use crate::capture::Capture;
use crate::comb::{count_permutations, pick};

#[derive(Clone, Debug)]
pub struct UnivariateDescentConfig {
    pub init_value: f64,
    pub init_step: f64,
    pub min_step: f64,
    pub max_steps: u64,
    pub acceptable_residual: f64,
}
impl UnivariateDescentConfig {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.min_step <= 0.0 {
            bail!("min step must be positive")
        }
        if self.acceptable_residual < 0.0 {
            bail!("acceptable residual must be non-negative")
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UnivariateDescentOutcome {
    pub steps: u64,
    pub optimal_value: f64,
    pub optimal_residual: f64,
}

/// Univariate, derivative-free search.
pub fn univariate_descent(
    config: &UnivariateDescentConfig,
    mut loss_f: impl FnMut(f64) -> f64,
) -> UnivariateDescentOutcome {
    config.validate().unwrap();

    let mut steps = 0;
    let mut residual = loss_f(config.init_value);
    if residual <= config.acceptable_residual {
        return UnivariateDescentOutcome {
            steps: 0,
            optimal_value: config.init_value,
            optimal_residual: residual
        };
    }

    let (mut value, mut step) = (config.init_value, config.init_step);
    // println!("initial value: {value}, residual: {residual}, step: {step}");
    let (mut optimal_value, mut optimal_residual) = (value, residual);
    // let mut boost = 1.0;
    // let mut gradient: f64 = 1.0;
    while steps < config.max_steps {
        steps += 1;
        let new_value = value + step;/* * boost*/ // * f64::min(gradient.abs(), 100.0);
        let new_residual = loss_f(new_value);
        // let gradient = (new_residual - residual) / (new_value - value);
        // println!("iterations: {iterations}, value: {value}, residual: {residual}, step: {step}, new_value: {new_value}, new_residual: {new_residual}");

        if new_residual > residual {
            step = -step * 0.5;
            if step.abs() < config.min_step {
                break;
            }
        } else if new_residual < optimal_residual {
            // boost = f64::min(gradient.abs(), 10.0);
            // println!("optimal_residual: {optimal_residual}, new_residual: {new_residual}, boost: {boost}, diff: {}", optimal_residual - new_residual);
            optimal_residual = new_residual;
            optimal_value = new_value;

            if optimal_residual <= config.acceptable_residual {
                break;
            }
        }
        residual = new_residual;
        value = new_value;
    }
    UnivariateDescentOutcome {
        steps,
        optimal_value,
        optimal_residual,
    }
}

#[derive(Clone, Debug)]
pub struct HypergridSearchConfig<'a> {
    pub max_steps: u64,
    pub acceptable_residual: f64,
    pub bounds: Capture<'a, Vec<RangeInclusive<f64>>, [RangeInclusive<f64>]>,
    pub resolution: usize,
}
impl HypergridSearchConfig<'_> {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.max_steps <= 0 {
            bail!("at least one step must be specified")
        }
        if self.acceptable_residual < 0.0 {
            bail!("acceptable residual must be non-negative")
        }
        if self.bounds.is_empty() {
            bail!("at least one search dimension must be specified")
        }
        const MIN_RESOLUTION: usize = 3;
        if self.resolution < MIN_RESOLUTION {
            bail!("search resolution must be at least {MIN_RESOLUTION}")
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct HypergridSearchOutcome {
    pub steps: u64,
    pub optimal_values: Vec<f64>,
    pub optimal_residual: f64,
}

pub fn hypergrid_search(
    config: &HypergridSearchConfig,
    mut constraint_f: impl FnMut(&[f64]) -> bool,
    mut loss_f: impl FnMut(&[f64]) -> f64) -> HypergridSearchOutcome {
    config.validate().unwrap();

    let mut steps = 0;
    let mut values = Vec::with_capacity(config.bounds.len());
    values.resize(values.capacity(), 0.0);

    let mut optimal_values = values.clone();
    let mut optimal_residual = f64::MAX;

    let cardinalities = {
        let mut cardinalities = Vec::with_capacity(values.len());
        cardinalities.resize(cardinalities.capacity(), config.resolution);
        cardinalities
    };
    let mut ordinals = cardinalities.clone();
    let permutations = count_permutations(&cardinalities);
    let mut bounds = (*config.bounds).to_vec();
    let inv_resolution = 1.0 / (config.resolution - 1) as f64;

    'outer: while steps < config.max_steps {
        // println!("step: {steps}, bounds: {bounds:?}");
        steps += 1;

        for permutation in 0.. permutations {
            pick(&cardinalities, permutation, &mut ordinals);

            for (dimension, &ordinal) in ordinals.iter().enumerate() {
                let bound = &bounds[dimension];
                let range = bound.end() - bound.start();
                values[dimension] = bound.start() + ordinal as f64 * range * inv_resolution;
                if constraint_f(&values) {
                    let residual = loss_f(&values);
                    // println!("  values: {values:?}, residual: {residual}");
                    if residual < optimal_residual {
                        // println!("    new optimal");
                        optimal_residual = residual;
                        optimal_values.copy_from_slice(&values);

                        if residual <= config.acceptable_residual {
                            break 'outer;
                        }
                    }
                }
            }
        }

        for (dimension, &value) in optimal_values.iter().enumerate() {
            let hard_bound = &config.bounds[dimension];
            let bound = &mut bounds[dimension];
            let new_range = (bound.end() - bound.start()) / config.resolution as f64;
            let new_start = f64::max(*hard_bound.start(), value - new_range / 2.0);
            let new_end = f64::min(new_start + new_range, *hard_bound.end());
            // let new_start = value - new_range / 2.0;
            // let new_end = new_start + new_range;
            *bound = new_start..=new_end;
        }
    }

    HypergridSearchOutcome {
        steps,
        optimal_values,
        optimal_residual,
    }
}

#[cfg(test)]
mod tests;
