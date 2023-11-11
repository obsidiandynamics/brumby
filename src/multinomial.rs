//! Multinomial distributions.

use crate::factorial::Factorial;

/// Binomial coefficient: the number of combinations obtained when sampling `r` items from a
/// set of `n` without replacement.
pub fn combinations(n: u8, r: u8, factorial: &impl Factorial) -> u128 {
    assert!(n >= r, "n ({n}) < r ({r})");
    factorial.get(n) / factorial.get(r) / factorial.get(n - r)
}

/// Probability of `r` successes in `n` independent Bernoulli trials, given `p` probability of success.
pub fn binomial(n: u8, r: u8, p: f64, factorial: &impl Factorial) -> f64 {
    assert!(n >= r, "n ({n}) < r ({r})");
    assert!(p <= 1.0, "p ({p}) > 1.0");
    combinations(n, r, factorial) as f64 * p.powi(r as i32) * (1.0 - p).powi((n - r) as i32)
}

pub fn trinomial(n: u8, r_1: u8, r_2: u8, p_1: f64, p_2: f64, factorial: &impl Factorial) -> f64 {
    assert!(r_1 + r_2 <= n, "r_1 ({r_1}) + r_2 ({r_2}) > n ({n})");
    assert!(p_1 + p_2 <= 1.0, "p_1 ({p_1}) + p_2 ({p_2}) + > 1.0");
    let p_3 = 1.0 - p_1 - p_2;
    let r_3 = n - r_1 - r_2;
    (factorial.get(n) / factorial.get(r_1) / factorial.get(r_2) / factorial.get(r_3)) as f64 * p_1.powi(r_1 as i32) * p_2.powi(r_2 as i32) * p_3.powi(r_3 as i32)
}

pub fn quadranomial(n: u8, r_1: u8, r_2: u8, r_3: u8, p_1: f64, p_2: f64, p_3: f64, factorial: &impl Factorial) -> f64 {
    assert!(r_1 + r_2 + r_3 <= n, "r_1 ({r_1}) + r_2 ({r_2}) + r_3 ({r_3}) > n ({n})");
    assert!(p_1 + p_2 + p_3 <= 1.0, "p_1 ({p_1}) + p_2 ({p_2}) + p_3 ({p_3}) > 1.0");
    let p_4 = 1.0 - p_1 - p_2 - p_3;
    let r_4 = n - r_1 - r_2 - r_3;
    (factorial.get(n) / factorial.get(r_1) / factorial.get(r_2) / factorial.get(r_3) / factorial.get(r_4)) as f64 * p_1.powi(r_1 as i32) * p_2.powi(r_2 as i32) * p_3.powi(r_3 as i32) * p_4.powi(r_4 as i32)
}

pub fn bivariate_binomial(n: u8, r_1: u8, r_2: u8, p_1: f64, p_2: f64, p_3: f64, factorial: &impl Factorial) -> f64 {
    assert!(r_1 <= n, "r_1 ({}) > n ({})", r_1, n);
    assert!(r_2 <= n, "r_2 ({}) > n ({})", r_2, n);
    assert!(p_1 + p_2 + p_3 <= 1.0, "p_1 ({p_1}) + p_2 ({p_2}) + p_3 ({p_3}) > 1.0");
    let rewind = u8::min(r_1, r_2);
    let mut prob = 0.0;
    let excess = if r_1 + r_2 > n { (r_1 + r_2 - n).div_ceil(2) } else { 0 };
    for i in excess..=rewind {
        let (k_1, k_2) = (r_1 - i, r_2 - i);
        // if k_1 + k_2 + i > n {
        //     continue
        // }
        let cell_prob = quadranomial(n, k_1, k_2, i, p_1, p_2, p_3, factorial);
        println!("n={n}, k_1:k_2={k_1}:{k_2}, i={i}, cell_prob={cell_prob}");
        prob += cell_prob;
    }
    prob
}


#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use crate::factorial::Calculator;
    use super::*;

    #[test]
    fn test_combinations() {
        assert_eq!(5, combinations(5, 1, &Calculator));
        assert_eq!(1, combinations(5, 5, &Calculator));
        assert_eq!(10, combinations(5, 3, &Calculator));
        assert_eq!(120, combinations(10, 3, &Calculator));
    }

    #[test]
    fn test_binomial() {
        assert_eq!(0.25, binomial(4, 1, 0.5, &Calculator));
        assert_eq!(0.421875, binomial(4, 1, 0.25, &Calculator));
        assert_eq!(0.375, binomial(4, 2, 0.5, &Calculator));
        assert_eq!(0.2109375, binomial(4, 2, 0.25, &Calculator));
        assert_eq!(0.046875, binomial(4, 3, 0.25, &Calculator));
        assert_eq!(0.421875, binomial(4, 3, 0.75, &Calculator));
    }

    /// Test limiting cases of the bivariate binomial when the interaction probability `XY = p_3` equals
    /// the product of the two outcome probabilities `X = p_1` and `Y = p_2`. I.e., when X and Y are
    /// independent.
    #[test]
    fn bivariate_binomial_independent() {
        fn test(n: u8, r_1: u8, r_2: u8, i_1: f64) {
            let i_2 = 1.0 - i_1;
            // probabilities for X, Y and XY
            let p_1 = i_1 * (1.0 - i_2);
            let p_2 = i_2 *(1.0 - i_1);
            let p_3 = i_1 * i_2;
            println!("n={n}, r_1={r_1}, r_2={r_2}, i_1={i_1}, p_1={p_1}, p_2={p_2}, p_3={p_3}");

            let independent_prob = binomial(n, r_1, i_1, &Calculator) * binomial(n, r_2, i_2, &Calculator);
            assert_eq!(independent_prob, bivariate_binomial(n, r_1, r_2, p_1, p_2, p_3, &Calculator));
        }
        for n in 0..4 {
            for r_1 in 0..n {
                for r_2 in 0..n {
                    test(n, r_1, r_2, 0.25);
                    if r_1 != r_2 {
                        test(n, r_2, r_1, 0.25);
                    }
                }
            }
        }
    }

    #[test]
    fn bivariate_binomial_dependent() {
        assert_float_absolute_eq!(0.04, bivariate_binomial(2, 0, 0, 0.25, 0.25, 0.3, &Calculator));
        assert_eq!(0.10, bivariate_binomial(2, 1, 0, 0.25, 0.25, 0.3, &Calculator));
        assert_eq!(0.10, bivariate_binomial(2, 0, 1, 0.25, 0.25, 0.3, &Calculator));
        assert_eq!(0.245, bivariate_binomial(2, 1, 1, 0.25, 0.25, 0.3, &Calculator));
    }

    #[test]
    fn test_trinomial() {
        // trinomial acts as a limiting case of a binomial when p_1 + p_2 = 1
        assert_eq!(0.25, trinomial(4, 1, 3, 0.5, 0.5, &Calculator));
        assert_eq!(0.421875, trinomial(4, 1, 3, 0.25, 0.75, &Calculator));
        assert_eq!(0.375, trinomial(4, 2, 2, 0.5, 0.5, &Calculator));
        assert_eq!(0.2109375, trinomial(4, 2, 2, 0.25, 0.75, &Calculator));
        assert_eq!(0.046875, trinomial(4, 3, 1, 0.25, 0.75, &Calculator));
        assert_eq!(0.421875, trinomial(4, 3, 1, 0.75, 0.25, &Calculator));

        // specific trinomial tests
        assert_eq!(0.25, trinomial(2, 1, 0, 0.25, 0.25, &Calculator));
        assert_eq!(0.015625, trinomial(3, 3, 0, 0.25, 0.5, &Calculator));
        assert_eq!(0.125, trinomial(3, 0, 3, 0.25, 0.5, &Calculator));
        assert_eq!(0.015625, trinomial(3, 0, 0, 0.25, 0.5, &Calculator));
        assert_eq!(0.09375, trinomial(3, 2, 1, 0.25, 0.5, &Calculator));
        assert_eq!(0.0, trinomial(4, 0, 2, 0.5, 0.5, &Calculator));
    }

    #[test]
    fn test_quadranomial() {
        // quadranomial acts as a limiting case of a binomial when p_1 + p_2 = 1
        assert_eq!(0.25, quadranomial(4, 1, 3, 0, 0.5, 0.5, 0.0,&Calculator));
        assert_eq!(0.421875, quadranomial(4, 1, 3, 0, 0.25, 0.75, 0.0, &Calculator));

        // quadranomial acts as a limiting case of a trinomial when p_1 + p_2 + p_3 = 1
        assert_eq!(0.25, quadranomial(2, 1, 0, 1, 0.25, 0.25, 0.5, &Calculator));
        assert_eq!(0.015625, quadranomial(3, 3, 0, 0, 0.25, 0.5, 0.25, &Calculator));
        assert_eq!(0.125, quadranomial(3, 0, 3, 0, 0.25, 0.5, 0.0, &Calculator));
        assert_eq!(0.015625, quadranomial(3, 0, 0, 3, 0.25, 0.5, 0.25, &Calculator));
        assert_eq!(0.09375, quadranomial(3, 2, 1, 0, 0.25, 0.5, 0.25, &Calculator));
        assert_eq!(0.0, quadranomial(4, 0, 2, 2,0.5, 0.5, 0.0, &Calculator));
    }
}