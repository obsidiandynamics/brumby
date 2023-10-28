//! Support for linear algebra.

use std::fmt::{Debug, Display, Formatter};
use std::ops::{Index, IndexMut};

#[derive(PartialEq, Clone, Debug)]
pub struct Matrix<T> {
    data: Vec<T>,
    rows: usize,
    cols: usize,
}
impl<T> Matrix<T> {
    pub const fn empty() -> Self {
        Self {
            data: vec![],
            rows: 0,
            cols: 0
        }
    }

    pub fn allocate(rows: usize, cols: usize) -> Self where T: Default {
        let (len, overflow) = rows.overflowing_mul(cols);
        assert!(
            !overflow,
            "allocation of a {rows}x{cols} matrix failed due to overflow"
        );
        let mut data = Vec::with_capacity(len);
        data.resize_with(len, T::default);
        Self { data, rows, cols }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn row_slice(&self, row: usize) -> &[T] {
        debug_assert!(self.validate_row_index(row));
        let row_start = row * self.cols;
        &self.data.as_slice()[row_start..(row_start + self.cols)]
    }

    pub fn row_slice_mut(&mut self, row: usize) -> &mut [T] {
        debug_assert!(self.validate_row_index(row));
        let row_start = row * self.cols;
        &mut self.data.as_mut_slice()[row_start..(row_start + self.cols)]
    }

    pub fn clone_row(&mut self, source_row: &[T]) where T: Copy {
        debug_assert_eq!(self.cols, source_row.len(), "length of source row {} does not match number of columns {}", source_row.len(), self.cols);
        for row in 0..self.rows {
            let row_slice = self.row_slice_mut(row);
            row_slice.copy_from_slice(source_row);
        }
    }

    pub fn verbose(&self) -> VerboseFormat<T> {
        VerboseFormat { referent: self }
    }

    pub fn unpack(self) -> (Vec<T>, usize, usize) {
        (self.data, self.rows, self.cols)
    }
    
    pub fn flatten(&self) -> &[T] {
        &self.data
    }

    pub fn flatten_mut(&mut self) -> &mut [T] {
        &mut self.data
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

impl<T> Display for Matrix<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for row in 0..self.rows {
            write!(f, "{:?}", self.row_slice(row))?;
        }
        Ok(())
    }
}

pub struct VerboseFormat<'a, T> {
    referent: &'a Matrix<T>,
}
impl<T> Display for VerboseFormat<'_, T> where T: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for row in 0..self.referent.rows {
            writeln!(f, "{:?}", self.referent.row_slice(row))?;
        }
        Ok(())
    }
}

impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        debug_assert!(self.validate_row_index(row));
        debug_assert!(self.validate_col_index(col));
        &self.data[row * self.cols + col]
    }
}

impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    #[inline]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (row, col) = index;
        debug_assert!(self.validate_row_index(row));
        debug_assert!(self.validate_col_index(col));
        &mut self.data[row * self.cols + col]
    }
}

impl<T> Index<usize> for Matrix<T> {
    type Output = [T];

    fn index(&self, row: usize) -> &Self::Output {
        self.row_slice(row)
    }
}

impl<T> IndexMut<usize> for Matrix<T> {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        self.row_slice_mut(row)
    }
}

#[cfg(test)]
pub(crate) mod matrix_fixtures {
    use super::*;

    pub fn populate_with_test_data(matrix: &mut Matrix<f64>) {
        for row in 0..matrix.rows() {
            for col in 0..matrix.cols() {
                matrix[(row, col)] = (row * matrix.cols() + col) as f64 * 10.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::linear::matrix_fixtures::populate_with_test_data;
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
    #[should_panic = "invalid row index 4 for a 4x3 matrix"]
    fn row_overflow() {
        let matrix = Matrix::<()>::allocate(4, 3);
        matrix[(matrix.rows(), 0)];
    }

    #[test]
    #[should_panic = "invalid column index 3 for a 4x3 matrix"]
    fn col_overflow() {
        let matrix = Matrix::<()>::allocate(4, 3);
        matrix[(0, matrix.cols())];
    }

    #[test]
    #[should_panic]
    fn allocate_overflow() {
        Matrix::<()>::allocate(usize::MAX, 2);
    }

    #[test]
    fn row_slice() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        assert_eq!(&[0.0, 10.0], &matrix[0]); // index access
        assert_eq!(&[20.0, 30.0], matrix.row_slice(1));
        assert_eq!(&[40.0, 50.0], matrix.row_slice(2));

        matrix[1][1] = 300.0; // index access
        assert_eq!(&[20.0, 300.0], matrix.row_slice(1));
    }

    #[test]
    #[should_panic = "invalid row index 3 for a 3x2 matrix"]
    fn row_slice_index_overflow() {
        let matrix = Matrix::<()>::allocate(3, 2);
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
    fn clone_row() {
        let mut matrix = Matrix::allocate(3, 2);
        matrix.clone_row(&[3.0, 4.0]);
        assert_eq!(&[3.0, 4.0, 3.0, 4.0, 3.0, 4.0], matrix.flatten());
    }

    #[test]
    fn flatten_mut() {
        let mut matrix = Matrix::allocate(3, 2);
        populate_with_test_data(&mut matrix);
        let flattened = matrix.flatten_mut();
        flattened[3] = 400.0;
        assert_eq!(400.0, matrix[(1, 1)]);
    }
}
