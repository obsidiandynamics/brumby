use crate::factorial;
use crate::factorial::Factorial;

#[inline]
pub fn univariate(k: u8, lambda: f64, factorial: &impl Factorial) -> f64 {
    lambda.powi(k as i32) * f64::exp(-lambda) / factorial.get(k) as f64
}

#[inline]
pub fn bivariate(
    k_1: u8,
    k_2: u8,
    lambda_1: f64,
    lambda_2: f64,
    lambda_3: f64,
    factorial: &impl Factorial,
) -> f64 {
    let sum = (0..=u8::min(k_1, k_2))
        .map(|i| {
            (factorial::combinations(k_1, i, factorial)
                * factorial::combinations(k_2, i, factorial)
                * factorial.get(i)) as f64
                * (lambda_3 / lambda_1 / lambda_2).powi(i as i32)
        })
        .sum::<f64>();
    f64::exp(-lambda_1 - lambda_2 - lambda_3) * lambda_1.powi(k_1 as i32) * lambda_2.powi(k_2 as i32) / (factorial.get(k_1) * factorial.get(k_2)) as f64 * sum
}

// pub fn lambda_to_interval_probability(lambda: f64, interval_ratio: f64, factorial: &impl Factorial) -> f64 {
//     let mut prob = 0.0;
//     for k in 1..5 {
//         prob = univariate(k, lambda * interval_ratio, factorial);
//     }
//     prob
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factorial::Calculator;
    use assert_float_eq::*;

    #[test]
    pub fn test_univariate() {
        assert_float_relative_eq!(
            0.36787944117144233,
            univariate(0, 1.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.36787944117144233,
            univariate(1, 1.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.18393972058572117,
            univariate(2, 1.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.0820849986238988,
            univariate(0, 2.5, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.205212496559747,
            univariate(1, 2.5, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.25651562069968376,
            univariate(2, 2.5, &Calculator::default())
        );
    }

    #[test]
    pub fn test_bivariate() {
        assert_float_relative_eq!(
            0.1353352832366127,
            bivariate(0, 0, 1.0, 1.0, 0.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.07549345856,
            bivariate(0, 1, 1.0, 2.5, 0.0, &Calculator::default())
        );
        // examples taken from https://gawhitaker.github.io/project.pdf — output of bivpois.table(6,6,lambda=c(2,1,3))
        assert_float_relative_eq!(
            0.0024787521766663585,
            bivariate(0, 0, 2.0, 1.0, 3.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.012393760883331793,
            bivariate(1, 1, 2.0, 1.0, 3.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.028505650031663124,
            bivariate(2, 2, 2.0, 1.0, 3.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.009915008706665434,
            bivariate(1, 2, 2.0, 1.0, 3.0, &Calculator::default())
        );
        assert_float_relative_eq!(
            0.019830017413330868,
            bivariate(2, 1, 2.0, 1.0, 3.0, &Calculator::default())
        );
    }
}
