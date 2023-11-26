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

const MAX_FACTORIAL_ENTRIES: usize = 35;

pub struct Lookup {
    entries: [u128; MAX_FACTORIAL_ENTRIES]
}
impl Factorial for Lookup {
    #[inline]
    fn get(&self, n: u8) -> u128 {
        self.entries[n as usize]
    }
}

impl Default for Lookup {
    fn default() -> Self {
        let mut entries = [1u128; MAX_FACTORIAL_ENTRIES];
        for i in 2..MAX_FACTORIAL_ENTRIES {
            entries[i] = i as u128 * entries[i - 1];
        }
        Self {
            entries
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn calculator() {
       test_impl(Calculator);
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
}