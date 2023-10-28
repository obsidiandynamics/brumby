use crate::overround::apply_with_cap;
use crate::probs::SliceExt;

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
    Multiplicative
}

#[derive(Debug)]
pub struct Market {
    pub probs: Vec<f64>,
    pub prices: Vec<f64>,
    pub overround: Overround
}
impl Market {
    pub fn fit(method: OverroundMethod, prices: Vec<f64>, norm_sum: f64) -> Self {
        match method {
            OverroundMethod::Multiplicative => Self::fit_multiplicative(prices, norm_sum)
        }
    }

    pub fn frame(method: OverroundMethod, probs: Vec<f64>, overround: f64) -> Self {
        match method {
            OverroundMethod::Multiplicative => Self::frame_multiplicative(probs, overround)
        }
    }

    fn fit_multiplicative(prices: Vec<f64>, norm_sum: f64) -> Self {
        let mut probs: Vec<_> = prices.invert().collect();
        let overround  = probs.normalise(norm_sum) / norm_sum;
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround,
            }
        }
    }

    fn frame_multiplicative(probs: Vec<f64>, overround: f64) -> Self {
        let prices: Vec<_> = probs.iter().map(|prob| apply_with_cap(1.0 / prob, overround)).collect();
        Self {
            probs,
            prices,
            overround: Overround {
                method: OverroundMethod::Multiplicative,
                value: overround
            }
        }
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
}

