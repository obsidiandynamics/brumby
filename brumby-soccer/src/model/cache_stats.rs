use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheStats {
    hits: usize,
    misses: usize,
}

impl Add<bool> for CacheStats {
    type Output = CacheStats;

    fn add(self, cache_hit: bool) -> Self::Output {
        if cache_hit {
            Self {
                hits: self.hits + 1,
                ..self
            }
        } else {
            Self {
                misses: self.misses + 1,
                ..self
            }
        }
    }
}

impl Add for CacheStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            hits: self.hits + rhs.hits,
            misses: self.misses + rhs.misses,
        }
    }
}

impl AddAssign<bool> for CacheStats {
    fn add_assign(&mut self, cache_hit: bool) {
        if cache_hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }
}

impl AddAssign for CacheStats {
    fn add_assign(&mut self, rhs: Self) {
        self.hits += rhs.hits;
        self.misses += rhs.misses;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_bool() {
        assert_eq!(CacheStats { hits: 1, misses: 0}, CacheStats::default() + true);
        assert_eq!(CacheStats { hits: 0, misses: 1}, CacheStats::default() + false);
    }

    #[test]
    fn add_assign_bool() {
        let mut cs = CacheStats { hits: 1, misses: 0};
        cs += false;
        assert_eq!(CacheStats { hits: 1, misses: 1}, cs);
        cs += true;
        assert_eq!(CacheStats { hits: 2, misses: 1}, cs);
    }

    #[test]
    fn add_self() {
        let cs = CacheStats { hits: 4, misses: 5};
        assert_eq!(CacheStats { hits: 7, misses: 6}, cs + CacheStats { hits: 3, misses: 1});
    }

    #[test]
    fn add_assign_self() {
        let mut cs = CacheStats { hits: 4, misses: 5};
        cs += CacheStats { hits: 3, misses: 1};
        assert_eq!(CacheStats { hits: 7, misses: 6}, cs);
    }
}