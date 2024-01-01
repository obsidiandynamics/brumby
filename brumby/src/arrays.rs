//! Utilities for working with arrays.

use thiserror::Error;
use crate::stack_vec;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FromIteratorError {
    #[error("{0}")]
    CapacityExceeded(#[from] stack_vec::CapacityExceeded),

    #[error("{0}")]
    IncompletelyFilled(#[from] stack_vec::IncompletelyFilled),
}

pub struct FromIteratorResult<T, const C: usize>(pub Result<[T; C], FromIteratorError>);

impl<T, const C: usize> FromIterator<T> for FromIteratorResult<T, C> {
    #[inline]
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        fn __from_iter<T, const C: usize, I: IntoIterator<Item=T>>(iter: I) -> Result<[T; C], FromIteratorError> {
            let stack_vec::FromIteratorResult::<T, C>(result) = iter.into_iter().collect();
            let sv = result?;
            let array = sv.to_array()?;
            Ok(array)
        }
        Self(__from_iter(iter))
    }
}

#[cfg(test)]
mod tests {
    use crate::stack_vec::{CapacityExceeded, IncompletelyFilled};
    use super::*;

    #[test]
    fn from_iterator_success() {
        let FromIteratorResult::<usize, 4>(result) = (0..4).into_iter().collect();
        assert_eq!([0, 1, 2, 3], result.unwrap());
    }

    #[test]
    fn from_iterator_capacity_exceeded() {
        let FromIteratorResult::<usize, 3>(result) = (0..4).into_iter().collect();
        assert_eq!(Err(FromIteratorError::CapacityExceeded(CapacityExceeded { capacity: 3 })), result);
    }

    #[test]
    fn from_iterator_incompletely_filled() {
        let FromIteratorResult::<usize, 5>(result) = (0..4).into_iter().collect();
        assert_eq!(Err(FromIteratorError::IncompletelyFilled(IncompletelyFilled { capacity: 5 })), result);
    }
}