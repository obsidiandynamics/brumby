//! Testing helpers.

use assert_float_eq::*;

pub fn assert_slice_f64_near(expected: &[f64], actual: &[f64], distance: u32) {
    assert_eq!(
        expected.len(),
        actual.len(),
        "lengths do not match: {} ≠ {}",
        expected.len(),
        actual.len()
    );
    for (index, &expected) in expected.iter().enumerate() {
        let actual = actual[index];
        if actual != expected {
            assert_f64_near!(expected, actual, distance);
        }
    }
}

pub fn assert_slice_f64_relative(expected: &[f64], actual: &[f64], epsilon: f64) {
    assert_eq!(
        expected.len(),
        actual.len(),
        "lengths do not match: {} ≠ {}",
        expected.len(),
        actual.len()
    );
    for (index, &expected) in expected.iter().enumerate() {
        let actual = actual[index];
        if actual != expected {
            assert_float_relative_eq!(expected, actual, epsilon);
        }
    }
}