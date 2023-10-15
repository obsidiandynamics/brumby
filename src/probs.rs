//! Utilities for working with probabilities.

use std::fmt::{Display, Formatter};

pub trait SliceExt {
    fn sum(&self) -> f64;
    fn normalize(&mut self) -> f64;
    fn dilate(&mut self, factor: f64);
    fn scale(&mut self, factor: f64);
}
impl SliceExt for [f64] {
    fn sum(&self) -> f64 {
        self.iter().sum()
    }

    fn normalize(&mut self) -> f64 {
        let sum = self.sum();
        self.scale(1.0/sum);
        sum
    }

    fn dilate(&mut self, factor: f64) {
        if factor >= 0.0 {
            dilate_pve(self, factor);
        } else {
            dilate_nve(self, factor);
        }
    }

    fn scale(&mut self, factor: f64) {
        for element in self {
            *element *= factor;
        }
    }
}

#[inline(always)]
fn dilate_pve(slice: &mut [f64], factor: f64) {
    let share = factor / slice.len() as f64;
    for element in slice {
        *element = (*element + share) / (1.0 + factor)
    }
}

#[inline(always)]
fn dilate_nve(slice: &mut [f64], factor: f64) {
    let share = factor / slice.len() as f64;
    let mut sum = 0.0;
    for element in &mut *slice {
        *element = f64::max(0.0, *element + share);
        sum += *element;
    }
    slice.scale(1.0 / sum);
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}
impl Fraction {
    pub fn quotient(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl Display for Fraction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::{afe_is_f64_near, afe_near_error_msg, assert_f64_near};

    #[test]
    fn sum() {
        let data = [0.0, 0.1, 0.2];
        assert_f64_near!(0.3, data.sum(), 1);
    }

    #[test]
    fn normalize() {
        let mut data = [0.05, 0.1, 0.15, 0.2];
        let sum = data.normalize();
        assert_f64_near!(0.5, sum, 1);
        assert_slice_f64_near(&[0.1, 0.2, 0.3, 0.4], &data, 1);
    }

    fn assert_slice_f64_near(expected: &[f64], actual: &[f64], distance: u32) {
        assert_eq!(
            expected.len(),
            actual.len(),
            "lengths do not match: {} ≠ {}",
            expected.len(),
            actual.len()
        );
        for (index, &value) in expected.iter().enumerate() {
            assert_f64_near!(value, actual[index], distance);
        }
    }
}
