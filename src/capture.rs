//! [Capture] is a minimalistic analogue of [Cow](std::borrow::Cow) that relaxes the [ToOwned] constrain while
//! supporting [?Sized](Sized) types. [CaptureMut] extends [Capture] with support for mutable references.

use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum Capture<'a, W: Borrow<B>, B: ?Sized> {
    Owned(W),
    Borrowed(&'a B),
}

impl<'a, W: Borrow<B> + Default, B: ?Sized> Default for Capture<'a, W, B> {
    fn default() -> Self {
        Self::Owned(W::default())
    }
}

impl<'a, W: Borrow<B>, B: ?Sized> Deref for Capture<'a, W, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            Capture::Owned(owned) => owned.borrow(),
            Capture::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, W: Borrow<B>, B: ?Sized> From<W> for Capture<'a, W, B> {
    fn from(value: W) -> Self {
        Self::Owned(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum CaptureMut<'a, W: BorrowMut<B>, B: ?Sized> {
    Owned(W),
    Borrowed(&'a mut B),
}

impl<'a, W: BorrowMut<B> + Default, B: ?Sized> Default for CaptureMut<'a, W, B> {
    fn default() -> Self {
        Self::Owned(W::default())
    }
}

impl<'a, W: BorrowMut<B>, B: ?Sized> Deref for CaptureMut<'a, W, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            CaptureMut::Owned(owned) => owned.borrow(),
            CaptureMut::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, W: BorrowMut<B>, B: ?Sized> DerefMut for CaptureMut<'a, W, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            CaptureMut::Owned(owned) => owned.borrow_mut(),
            CaptureMut::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, W: BorrowMut<B>, B: ?Sized> From<W> for CaptureMut<'a, W, B> {
    fn from(value: W) -> Self {
        Self::Owned(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NotClone;

    #[test]
    fn capture_not_clone() {
        Capture::Owned(NotClone);
    }

    #[test]
    fn capture_owned() {
        let capture: Capture<Vec<u32>, [u32]> = Capture::Owned(vec![10, 20, 30]);
        assert_eq!([10, 20, 30], *capture);
    }

    #[test]
    fn capture_borrowed() {
        let capture: Capture<Vec<u32>, [u32]> = Capture::Borrowed(&[10, 20, 30]);
        assert_eq!([10, 20, 30], *capture);
    }

    #[test]
    fn capture_mut_not_clone() {
        CaptureMut::Owned(NotClone);
    }

    #[test]
    fn capture_mut_owned() {
        let capture: CaptureMut<Vec<u32>, [u32]> = CaptureMut::Owned(vec![10, 20, 30]);
        assert_eq!([10, 20, 30], *capture);
    }

    #[test]
    fn capture_mut_borrowed() {
        let mut owned = vec![10, 20, 30];
        let mut capture: CaptureMut<Vec<u32>, [u32]> = CaptureMut::Borrowed(&mut owned);
        assert_eq!([10, 20, 30], *capture);
        capture[0] = 100;
        assert_eq!([100, 20, 30], *capture);
    }
}
