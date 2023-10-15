//! Support for linear algebra.

use std::ops::{Index, IndexMut};

pub struct Matrix {
    data: Vec<f64>,
    rows: usize,
    cols: usize
}
impl Matrix {
    pub fn allocate(rows: usize, cols: usize) -> Self {
        let (len, overflow) = rows.overflowing_mul(cols);
        assert!(!overflow, "allocation of a {rows}x{cols} matrix failed due to overflow");
        let data = vec![0.0; len];
        Self { data, rows, cols }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn row_slice(&self, row: usize) -> &[f64] {
        debug_assert!(self.validate_row_index(row));
        let row_start = row * self.cols;
        &self.data.as_slice()[row_start..(row_start + self.cols)]
    }

    pub fn row_slice_mut(&mut self, row: usize) -> &mut [f64] {
        debug_assert!(self.validate_row_index(row));
        let row_start = row * self.cols;
        &mut self.data.as_mut_slice()[row_start..(row_start + self.cols)]
    }

    fn validate_row_index(&self, row: usize) -> bool {
        assert!(row < self.rows, "invalid row index {row} for a {}x{} matrix", self.rows, self.cols);
        true
    }

    fn validate_col_index(&self, col: usize) -> bool {
        assert!(col < self.cols, "invalid column index {col} for a {}x{} matrix", self.rows, self.cols);
        true
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = f64;

    #[inline]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        debug_assert!(self.validate_row_index(row));
        debug_assert!(self.validate_col_index(col));
        &self.data[row * self.cols + col]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    #[inline]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (row, col) = index;
        debug_assert!(self.validate_row_index(row));
        debug_assert!(self.validate_col_index(col));
        &mut self.data[row * self.cols + col]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index() {
        let mut matrix = Matrix::allocate(4, 3);
        assert_eq!(4, matrix.rows());
        assert_eq!(3, matrix.cols());
        for row in 0..matrix.rows() {
            for col in 0..matrix.cols() {
                assert_eq!(0.0, matrix[(row, col)]);
                let new_val = (row * matrix.cols() + col) as f64 * 10.0;
                matrix[(row, col)] = new_val;
                assert_eq!(new_val, matrix[(row, col)]);
            }
        }
    }

    #[test]
    #[should_panic="invalid row index 4 for a 4x3 matrix"]
    fn row_overflow_panics() {
        let matrix = Matrix::allocate(4, 3);
        matrix[(matrix.rows(), 0)];
    }

    #[test]
    #[should_panic="invalid column index 3 for a 4x3 matrix"]
    fn col_overflow_panics() {
        let matrix = Matrix::allocate(4, 3);
        matrix[(0, matrix.cols())];
    }

    #[test]
    #[should_panic]
    fn allocate_overflow_panics() {
        Matrix::allocate(usize::MAX, 2);
    }

    #[test]
    fn row_slice() {
        let mut matrix = Matrix::allocate(4, 3);
        for row in 0..matrix.rows() {
            for col in 0..matrix.cols() {
                matrix[(row, col)] = (row * matrix.cols() + col) as f64 * 10.0;
            }
        }
        assert_eq!(&[0.0, 10.0, 20.0], matrix.row_slice(0));
    }
}