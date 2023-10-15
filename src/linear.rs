//! Support for linear algebra.

use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};
use crate::probs::SliceExt;

#[derive(Debug, PartialEq, Clone)]
pub struct Matrix {
    data: Vec<f64>,
    rows: usize,
    cols: usize,
}
impl Matrix {
    pub fn allocate(rows: usize, cols: usize) -> Self {
        let (len, overflow) = rows.overflowing_mul(cols);
        assert!(
            !overflow,
            "allocation of a {rows}x{cols} matrix failed due to overflow"
        );
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

    pub fn scale_rows(&mut self, factors: &[f64]) {
        debug_assert_eq!(
            self.rows,
            factors.len(),
            "number of factors {} does not match number of rows {}",
            factors.len(),
            self.rows
        );
        for (row, &factor) in factors.iter().enumerate() {
            let row_slice = self.row_slice_mut(row);
            row_slice.scale(factor);
        }
    }

    pub fn verbose(&self) -> VerboseFormat {
        VerboseFormat { referent: self }
    }

    pub fn unpack(self) -> (Vec<f64>, usize, usize) {
        (self.data, self.rows, self.cols)
    }

    fn validate_row_index(&self, row: usize) -> bool {
        assert!(
            row < self.rows,
            "invalid row index {row} for a {}x{} matrix",
            self.rows,
            self.cols
        );
        true
    }

    fn validate_col_index(&self, col: usize) -> bool {
        assert!(
            col < self.cols,
            "invalid column index {col} for a {}x{} matrix",
            self.rows,
            self.cols
        );
        true
    }
}

impl Display for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for row in 0..self.rows {
            write!(f, "{:?}", self.row_slice(row))?;
        }
        Ok(())
    }
}

pub struct VerboseFormat<'a> {
    referent: &'a Matrix,
}
impl Display for VerboseFormat<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for row in 0..self.referent.rows {
            writeln!(f, "{:?}", self.referent.row_slice(row))?;
        }
        Ok(())
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

    fn populate_with_test_data(matrix: &mut Matrix) {
        for row in 0..matrix.rows() {
            for col in 0..matrix.cols() {
                matrix[(row, col)] = (row * matrix.cols() + col) as f64 * 10.0;
            }
        }
    }

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
    #[should_panic = "invalid row index 4 for a 4x3 matrix"]
    fn row_overflow() {
        let matrix = Matrix::allocate(4, 3);
        matrix[(matrix.rows(), 0)];
    }

    #[test]
    #[should_panic = "invalid column index 3 for a 4x3 matrix"]
    fn col_overflow() {
        let matrix = Matrix::allocate(4, 3);
        matrix[(0, matrix.cols())];
    }

    #[test]
    #[should_panic]
    fn allocate_overflow() {
        Matrix::allocate(usize::MAX, 2);
    }

    #[test]
    fn row_slice() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        assert_eq!(&[0.0, 10.0], matrix.row_slice(0));
        assert_eq!(&[20.0, 30.0], matrix.row_slice(1));
        assert_eq!(&[40.0, 50.0], matrix.row_slice(2));

        matrix.row_slice_mut(1)[1] = 300.0;
        assert_eq!(&[20.0, 300.0], matrix.row_slice(1));
    }

    #[test]
    #[should_panic = "invalid row index 3 for a 3x2 matrix"]
    fn row_slice_index_overflow() {
        let matrix = Matrix::allocate(3, 2);
        matrix.row_slice(matrix.rows());
    }

    #[test]
    fn display() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        let display = format!("{matrix}");
        assert_eq!("[0.0, 10.0][20.0, 30.0][40.0, 50.0]", display);
    }

    #[test]
    fn verbose_display() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        let display = format!("{}", matrix.verbose());
        assert_eq!(
            "[0.0, 10.0]\n\
            [20.0, 30.0]\n\
            [40.0, 50.0]\n",
            display
        );
    }

    #[test]
    fn unpack() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        let (data, rows, cols) = matrix.unpack();
        assert_eq!(3, rows);
        assert_eq!(2, cols);
        assert_eq!(&[0.0, 10.0, 20.0, 30.0, 40.0, 50.0], &*data);
    }

    #[test]
    fn scale_rows() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        matrix.scale_rows(&[2.0, 4.0, 6.0]);
        let (data, _, _) = matrix.unpack();
        assert_eq!(&[0.0, 20.0, 80.0, 120.0, 240.0, 300.0], &*data);
    }
}
