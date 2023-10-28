use crate::opt;
use crate::opt::GradientDescentConfig;
use crate::probs::SliceExt;

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
    pub value: f64
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverroundMethod {
    Multiplicative,
    Power
}

#[derive(Debug)]
pub struct Market {
    pub probs: Vec<f64>,
    pub prices: Vec<f64>,
    pub overround: Overround
}
impl Market {
    pub fn fit(method: OverroundMethod, prices: Vec<f64>, fair_sum: f64) -> Self {
        match method {
            OverroundMethod::Multiplicative => Self::fit_multiplicative(prices, fair_sum),
            OverroundMethod::Power => Self::fit_power(prices, fair_sum),
        }
    }

    pub fn frame(method: OverroundMethod, probs: Vec<f64>, overround: f64) -> Self {
        match method {
            OverroundMethod::Multiplicative => Self::frame_multiplicative(probs, overround),
            OverroundMethod::Power => Self::frame_power(probs, overround),
        }
    }

    fn fit_multiplicative(prices: Vec<f64>, fair_sum: f64) -> Self {
        let mut probs: Vec<_> = prices.invert().collect();
        let overround  = probs.normalise(fair_sum) / fair_sum;
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround,
            }
        }
    }

    fn fit_power(prices: Vec<f64>, fair_sum: f64) -> Market {
        let prob_sum = prices.sum();
        let rtp = 1.0 / fair_sum / prob_sum;
        let initial_k = 1.0 + f64::ln(rtp) / f64::ln(prices.len() as f64);
        todo!()
    }

    fn frame_multiplicative(probs: Vec<f64>, overround: f64) -> Self {
        let prices: Vec<_> = probs.iter().map(|prob| multiply_capped(1.0 / prob, overround)).collect();
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround
            }
        }
    }

    fn frame_power(probs: Vec<f64>, overround: f64) -> Market {
        let rtp = 1.0 / overround;
        let fair_sum = probs.sum();
        let initial_k = 1.0 + f64::ln(rtp) / f64::ln(probs.len() as f64);
        let outcome = opt::gd(GradientDescentConfig {
            init_value: initial_k,
            step: -0.01,
            min_step: 0.0001,
            max_steps: 1_000,
            max_residual: 1e-9
        }, |exponent| {
            let mut sum = 0.0;
            for &prob in &probs {
                let price = (prob / fair_sum).powf(-exponent);
                sum += 1.0 / price;
            }

            (sum - overround).powi(2)
        });
        println!("gd outcome: {outcome:?}");
        let prices: Vec<_> = probs.iter().map(|prob| (prob / fair_sum).powf(-outcome.optimal_value) / fair_sum).collect();
        Self {
            probs,
            prices,
            overround: Overround { method: OverroundMethod::Multiplicative, value: overround },
        }
    }
}

pub fn multiply_capped(fair_price: f64, overround: f64) -> f64 {
    let quotient = fair_price / overround;
    if quotient.is_finite() {
        f64::min(f64::max(MIN_PRICE, quotient), MAX_PRICE)
    } else {
        quotient
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use crate::testing::assert_slice_f64_relative;
    use super::*;

    #[test]
    fn fit_multiplicative() {
        {
            let prices = vec![10.0, 5.0, 3.333, 2.5];
            let market = Market::fit(OverroundMethod::Multiplicative, prices, 1.0);
            assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
            assert_float_absolute_eq!(1.0, 1.0, 0.001);
        }
        {
            let prices = vec![9.0909, 4.5454, 3.0303, 2.273];
            let market = Market::fit(OverroundMethod::Multiplicative, prices, 1.0);
            println!("market: {:?}", market);
            assert_slice_f64_relative(&[0.1, 0.2, 0.3, 0.4], &market.probs, 0.001);
            assert_float_absolute_eq!(1.1, 1.1, 0.001);
        }
        {
            let prices = vec![4.5454, 2.2727, 1.5152, 1.1364];
            let market = Market::fit(OverroundMethod::Multiplicative, prices, 2.0);
            println!("market: {:?}", market);
            assert_slice_f64_relative(&[0.2, 0.4, 0.6, 0.8], &market.probs, 0.001);
            assert_float_absolute_eq!(1.1, 1.1, 0.001);
        }
    }

    #[test]
    fn frame_multiplicative() {
        {
            let probs = vec![0.1, 0.2, 0.3, 0.4];
            let market = Market::frame(OverroundMethod::Multiplicative, probs, 1.0);
            assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
        }
        {
            let probs = vec![0.1, 0.2, 0.3, 0.4];
            let market = Market::frame(OverroundMethod::Multiplicative, probs, 1.1);
            assert_slice_f64_relative(&[9.0909, 4.5454, 3.0303, 2.273], &market.prices, 0.001);
        }
        {
            let probs = vec![0.2, 0.4, 0.6, 0.8];
            let market = Market::frame(OverroundMethod::Multiplicative, probs, 1.1);
            assert_slice_f64_relative(&[4.5454, 2.2727, 1.5152, 1.1364], &market.prices, 0.001);
        }
    }

    #[test]
    fn frame_power() {
        {
            let probs = vec![0.1, 0.2, 0.3, 0.4];
            let market = Market::frame(OverroundMethod::Power, probs, 1.0);
            println!("market: {:?}", market);
            assert_slice_f64_relative(&[10.0, 5.0, 3.333, 2.5], &market.prices, 0.001);
        }
        {
            let probs = vec![0.1, 0.2, 0.3, 0.4];
            let market = Market::frame(OverroundMethod::Power, probs, 1.1);
            println!("market: {:?}", market);
            assert_slice_f64_relative(&[8.4319, 4.4381, 3.0489, 2.3359], &market.prices, 0.001);
        }
        {
            let probs = vec![0.2, 0.4, 0.6, 0.8];
            let market = Market::frame(OverroundMethod::Power, probs, 1.1);
            println!("market: {:?}", market);
            assert_slice_f64_relative(&[4.2159, 2.219, 1.5244, 1.168], &market.prices, 0.001);
        }
    }
}

