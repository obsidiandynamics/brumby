//! Utilities for working with probabilities.

use crate::linear::Matrix;
use std::fmt::{Display, Formatter};

pub trait SliceExt {
    fn sum(&self) -> f64;
    fn normalize(&mut self) -> f64;
    fn dilate_additive(&mut self, factor: f64);
    fn scale(&mut self, factor: f64);
    fn dilate_rows_additive(&self, matrix: &mut Matrix);
}
impl SliceExt for [f64] {
    fn sum(&self) -> f64 {
        self.iter().sum()
    }

    fn normalize(&mut self) -> f64 {
        let sum = self.sum();
        self.scale(1.0 / sum);
        sum
    }

    fn dilate_additive(&mut self, factor: f64) {
        if factor >= 0.0 {
            dilate_additive_pve(self, factor);
        } else {
            dilate_additive(self, factor);
        }
    }

    fn scale(&mut self, factor: f64) {
        for element in self {
            *element *= factor;
        }
    }

    fn dilate_rows_additive(&self, matrix: &mut Matrix) {
        debug_assert_eq!(
            self.len(),
            matrix.rows(),
            "number of dilation factors {} must match the number of matrix rows {}",
            self.len(),
            matrix.rows()
        );
        for (row, factor) in self.iter().enumerate() {
            let row_slice = matrix.row_slice_mut(row);
            row_slice.dilate_additive(*factor);
        }
    }
}

#[inline(always)]
fn dilate_additive_pve(slice: &mut [f64], factor: f64) {
    let share = factor / slice.len() as f64;
    for element in slice {
        *element = (*element + share) / (1.0 + factor)
    }
}

#[inline(always)]
fn dilate_additive(slice: &mut [f64], factor: f64) {
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
    use assert_float_eq::*;

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

    #[test]
    fn dilate_additive_zero() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_additive(0.0);
        assert_slice_f64_near(&[0.1, 0.2, 0.3, 0.4], &data, 1);
    }

    #[test]
    fn dilate_additive_pve() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_additive(0.2);
        assert_slice_f64_relative(&[0.125, 0.2083, 0.2917, 0.375], &data, 0.01);
    }

    #[test]
    fn dilate_additive_nve() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_additive(-0.2);
        assert_slice_f64_relative(&[0.0625, 0.1875, 0.3125, 0.4375], &data, 0.01);
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

    fn assert_slice_f64_relative(expected: &[f64], actual: &[f64], epsilon: f64) {
        assert_eq!(
            expected.len(),
            actual.len(),
            "lengths do not match: {} ≠ {}",
            expected.len(),
            actual.len()
        );
        for (index, &value) in expected.iter().enumerate() {
            assert_float_relative_eq!(value, actual[index], epsilon);
        }
    }
}
