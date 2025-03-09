//! [`Capture`] is a minimalistic analogue of [`Cow`](std::borrow::Cow) that relaxes the [`ToOwned`] constrain while
//! supporting [`?Sized`](Sized) types. [`CaptureMut`] extends [`Capture`] with support for mutable references.

use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq)]
pub enum Capture<'a, W: Borrow<B>, B: ?Sized = W> {
    Owned(W),
    Borrowed(&'a B),
}

impl<W: Borrow<B> + Default, B: ?Sized> Default for Capture<'_, W, B> {
    fn default() -> Self {
        Self::Owned(W::default())
    }
}

impl<W: Borrow<B>, B: ?Sized> Deref for Capture<'_, W, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            Capture::Owned(owned) => owned.borrow(),
            Capture::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<W: Borrow<B>, B: ?Sized> From<W> for Capture<'_, W, B> {
    fn from(value: W) -> Self {
        Self::Owned(value)
    }
}

impl<B: ?Sized + ToOwned> Clone for Capture<'_, B::Owned, B> {
    fn clone(&self) -> Self {
        match self {
            Capture::Owned(owned) => Self::Owned(owned.borrow().to_owned()),
            Capture::Borrowed(borrowed) => Self::Borrowed(borrowed),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CaptureMut<'a, W: BorrowMut<B>, B: ?Sized = W> {
    Owned(W),
    Borrowed(&'a mut B),
}

impl<W: BorrowMut<B> + Default, B: ?Sized> Default for CaptureMut<'_, W, B> {
    fn default() -> Self {
        Self::Owned(W::default())
    }
}

impl<W: BorrowMut<B>, B: ?Sized> Deref for CaptureMut<'_, W, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            CaptureMut::Owned(owned) => owned.borrow(),
            CaptureMut::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<W: BorrowMut<B>, B: ?Sized> DerefMut for CaptureMut<'_, W, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            CaptureMut::Owned(owned) => owned.borrow_mut(),
            CaptureMut::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<W: BorrowMut<B>, B: ?Sized> From<W> for CaptureMut<'_, W, B> {
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
    fn capture_clone() {
        assert_eq!(
            Capture::Owned(vec!["abc"]),
            Capture::<Vec<&str>, [&str]>::Owned(vec!["abc"]).clone()
        );
        assert_eq!(
            Capture::Borrowed(vec!["abc"].as_slice()),
            Capture::<Vec<&str>, [&str]>::Borrowed(vec!["abc"].as_slice()).clone()
        );
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
