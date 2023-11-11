pub trait Factorial {
    fn get(&self, n: u8) -> u128;
}

#[derive(Default)]
pub struct Calculator;

impl Factorial for Calculator {
    #[inline]
    fn get(&self, n: u8) -> u128 {
        assert!(n <= 34, "{n}! overflows");
        let mut product = 1u128;
        for i in 2..=n {
            product *= i as u128;
        }
        product
    }
}

const MAX_ENTRIES: usize = 35;

pub struct Lookup {
    entries: [u128; MAX_ENTRIES]
}
impl Factorial for Lookup {
    #[inline]
    fn get(&self, n: u8) -> u128 {
        self.entries[n as usize]
    }
}

impl Default for Lookup {
    fn default() -> Self {
        let mut entries = [1u128; MAX_ENTRIES];
        for i in 2..MAX_ENTRIES {
            entries[i] = i as u128 * entries[i - 1];
        }
        Self {
            entries
        }
    }
}

/// Binomial coefficient: the number of combinations obtained when sampling `r` items from a
/// set of `n` without replacement.
pub fn combinations(n: u8, r: u8, factorial: &impl Factorial) -> u128 {
    assert!(n >= r, "n ({n}) < r ({r})");
    factorial.get(n) / factorial.get(r) / factorial.get(n - r)
}

/// Probability of `r` successes in `n` independent Bernoulli trials, given `p` probability of success.
pub fn binomial(n: u8, r: u8, p: f64, factorial: &impl Factorial) -> f64 {
    combinations(n, r, factorial) as f64 * p.powi(r as i32) * (1.0 - p).powi((n - r) as i32)
}

pub fn constrained_bivariate_binomial(n: u8, r_1: u8, r_2: u8, p_1: f64, p_2: f64, factorial: &impl Factorial) -> f64 {
    (factorial.get(n) / factorial.get(r_1) / factorial.get(r_2)) as f64 * p_1.powi(r_1 as i32) * p_2.powi(r_2 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn calculator() {
       test_impl(Calculator::default());
    }

    #[test]
    pub fn lookup() {
        test_impl(Lookup::default());
    }

    fn test_impl(f: impl Factorial) {
        assert_eq!(1, f.get(0));
        assert_eq!(1, f.get(1));
        assert_eq!(2, f.get(2));
        assert_eq!(6, f.get(3));
        assert_eq!(24, f.get(4));
        assert_eq!(3_628_800, f.get(10));
    }

    #[test]
    fn test_combinations() {
        assert_eq!(5, combinations(5, 1, &Calculator::default()));
        assert_eq!(1, combinations(5, 5, &Calculator::default()));
        assert_eq!(10, combinations(5, 3, &Calculator::default()));
        assert_eq!(120, combinations(10, 3, &Calculator::default()));
    }

    #[test]
    fn test_binomial() {
        assert_eq!(0.25, binomial(4, 1, 0.5, &Calculator::default()));
        assert_eq!(0.421875, binomial(4, 1, 0.25, &Calculator::default()));
        assert_eq!(0.375, binomial(4, 2, 0.5, &Calculator::default()));
        assert_eq!(0.2109375, binomial(4, 2, 0.25, &Calculator::default()));
        assert_eq!(0.046875, binomial(4, 3, 0.25, &Calculator::default()));
        assert_eq!(0.421875, binomial(4, 3, 0.75, &Calculator::default()));
    }

    #[test]
    fn test_bivariate_binomial() {
        assert_eq!(0.5, constrained_bivariate_binomial(2, 1, 1, 0.5, 0.5, &Calculator::default()));
        assert_eq!(0.375, constrained_bivariate_binomial(4, 2, 2, 0.5, 0.5, &Calculator::default()));
        assert_eq!(0.421875, constrained_bivariate_binomial(4, 1, 3, 0.25, 0.75, &Calculator::default()));
    }
}