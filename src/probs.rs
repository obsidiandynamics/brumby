//! Utilities for working with probabilities.

use crate::linear::Matrix;
use std::fmt::{Display, Formatter};

pub trait SliceExt {
    fn sum(&self) -> f64;
    fn normalise(&mut self, target: f64) -> f64;
    fn geometric_mean(&self) -> f64;
    fn dilate_additive(&mut self, factor: f64);
    fn dilate_power(&mut self, factor: f64);
    fn scale(&mut self, factor: f64);
    fn scale_rows(&self, target: &mut Matrix<f64>);
    fn dilate_rows_additive(&self, matrix: &mut Matrix<f64>);
    fn dilate_rows_power(&self, matrix: &mut Matrix<f64>);
}
impl SliceExt for [f64] {
    fn sum(&self) -> f64 {
        self.iter().sum()
    }

    fn normalise(&mut self, target: f64) -> f64 {
        let sum = self.sum();
        self.scale(target / sum);
        sum
    }

    fn geometric_mean(&self) -> f64 {
        let product: f64 = self.iter().product();
        product.powf(1.0 / self.len() as f64)
    }

    fn dilate_additive(&mut self, factor: f64) {
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

        if factor >= 0.0 {
            dilate_additive_pve(self, factor);
        } else {
            dilate_additive(self, factor);
        }
    }

    fn dilate_power(&mut self, factor: f64) {
        let mut sum = 0.0;
        for element in &mut *self {
            // if *element > 0.02 {//TODO
            *element = element.powf(1.0 - factor);
            // }
            sum += *element;
        }
        self.scale(1.0 / sum);
    }

    fn scale(&mut self, factor: f64) {
        for element in self {
            *element *= factor;
        }
    }

    fn scale_rows(&self, target: &mut Matrix<f64>) {
        debug_assert_eq!(
            target.rows(),
            self.len(),
            "number of factors {} does not match number of rows {}",
            self.len(),
            target.rows()
        );
        for (row, &factor) in self.iter().enumerate() {
            let row_slice = target.row_slice_mut(row);
            row_slice.scale(factor);
        }
    }

    fn dilate_rows_additive(&self, matrix: &mut Matrix<f64>) {
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

    fn dilate_rows_power(&self, matrix: &mut Matrix<f64>) {
        debug_assert_eq!(
            self.len(),
            matrix.rows(),
            "number of dilation factors {} must match the number of matrix rows {}",
            self.len(),
            matrix.rows()
        );
        for (row, factor) in self.iter().enumerate() {
            let row_slice = matrix.row_slice_mut(row);
            row_slice.dilate_power(*factor);
        }
    }
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
    use crate::linear::matrix_fixtures::populate_with_test_data;

    #[test]
    fn sum() {
        let data = [0.0, 0.1, 0.2];
        assert_f64_near!(0.3, data.sum(), 1);
    }

    #[test]
    fn normalise() {
        let mut data = [0.05, 0.1, 0.15, 0.2];
        let sum = data.normalise(1.0);
        assert_f64_near!(0.5, sum, 1);
        assert_slice_f64_near(&[0.1, 0.2, 0.3, 0.4], &data, 1);
    }

    #[test]
    fn scale_rows() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        [2.0, 4.0, 6.0].scale_rows(&mut matrix);
        assert_eq!(&[0.0, 20.0, 80.0, 120.0, 240.0, 300.0], matrix.flatten());
    }

    #[test]
    fn geometric_mean() {
        let data = [1.0, 3.0, 9.0];
        assert_f64_near!(3.0, data.geometric_mean());
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
        assert_slice_f64_relative(&[0.125, 0.2083, 0.2917, 0.375], &data, 0.0005);
    }

    #[test]
    fn dilate_additive_nve() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_additive(-0.2);
        assert_slice_f64_relative(&[0.0625, 0.1875, 0.3125, 0.4375], &data, 0.0005);
    }

    #[test]
    fn dilate_power_zero() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_power(0.0);
        assert_slice_f64_near(&[0.1, 0.2, 0.3, 0.4], &data, 1);
    }

    #[test]
    fn dilate_power_pve() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_power(0.2);
        assert_slice_f64_relative(&[0.1222, 0.2128, 0.2944, 0.3706], &data, 0.0005);
    }

    #[test]
    fn dilate_power_nve() {
        let mut data = [0.1, 0.2, 0.3, 0.4];
        data.dilate_power(-0.2);
        assert_slice_f64_relative(&[0.0812, 0.1866, 0.3035, 0.4287], &data, 0.0005);
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
