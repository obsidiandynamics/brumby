use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};

pub enum MaybeOwned<'a, B: ToOwned + ?Sized + 'a> {
    Owned(B::Owned),
    Borrowed(&'a B),
}

impl<'a, B: ToOwned + ?Sized + 'a> Deref for MaybeOwned<'a, B>
    where
        B::Owned: Borrow<B>,
{
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeOwned::Owned(owned) => owned.borrow(),
            MaybeOwned::Borrowed(borrowed) => borrowed,
        }
    }
}

pub enum MaybeOwnedMut<'a, B: ToOwned + ?Sized + 'a> {
    Owned(B::Owned),
    Borrowed(&'a mut B),
}

impl<'a, B: ToOwned + ?Sized + 'a> Deref for MaybeOwnedMut<'a, B>
    where
        B::Owned: Borrow<B>,
{
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeOwnedMut::Owned(owned) => owned.borrow(),
            MaybeOwnedMut::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, B: ToOwned + ?Sized + 'a> DerefMut for MaybeOwnedMut<'a, B>
    where
        B::Owned: BorrowMut<B>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeOwnedMut::Owned(owned) => owned.borrow_mut(),
            MaybeOwnedMut::Borrowed(borrowed) => borrowed,
        }
    }
}

pub enum MaybeOwnedMutSized<'a, B> {
    Owned(B),
    Borrowed(&'a mut B),
}

impl<'a, B> Deref for MaybeOwnedMutSized<'a, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeOwnedMutSized::Owned(owned) => &owned,
            MaybeOwnedMutSized::Borrowed(borrowed) => borrowed,
        }
    }
}

impl<'a, B> DerefMut for MaybeOwnedMutSized<'a, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeOwnedMutSized::Owned(owned) => owned,
            MaybeOwnedMutSized::Borrowed(borrowed) => borrowed,
        }
    }
}