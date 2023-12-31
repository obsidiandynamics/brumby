use std::{mem, ptr, slice};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};

pub struct RawArray<T, const C: usize> {
    array: [MaybeUninit<T>; C]
}
impl<T, const C: usize> RawArray<T, C> {
    /// # Safety
    ///
    /// This function should not be called if location at `index` is uninitialised.
    #[inline(always)]
    pub unsafe fn get(&self, index: usize) -> &T {
        &*self.array[index].as_ptr()
    }

    /// # Safety
    ///
    /// This function should not be called if location at `index` is uninitialised.
    #[inline(always)]
    pub unsafe fn get_mut(&mut self, index: usize) -> &mut T {
        &mut *self.array[index].as_mut_ptr()
    }

    /// # Safety
    ///
    /// This function does not clean up the existing item and should not be called if
    /// location at `index` is already initialised.
    #[inline(always)]
    pub unsafe fn set_and_forget(&mut self, index: usize, value: T) {
        let ptr = self.array[index].as_mut_ptr();
        ptr.write(value);
    }

    /// # Safety
    ///
    /// This function should not be called if location at `index` is uninitialised.
    #[inline(always)]
    pub unsafe fn take(&mut self, index: usize) -> T {
        let ptr = self.array[index].as_ptr();
        ptr.read()
    }

    /// # Safety
    ///
    /// This function should not be called if the locations in the range `offset..offset + len` are
    /// uninitialised.
    #[inline(always)]
    pub unsafe fn drop_range(&mut self, offset: usize, len: usize) {
        if mem::needs_drop::<T>() {
            for index in offset..offset + len {
                let ptr = self.array[index].as_mut_ptr();
                ptr.drop_in_place();
            }
        }
    }

    /// # Safety
    ///
    /// This function should not be called if the locations in the range `offset..offset + len` are
    /// uninitialised.
    #[inline(always)]
    unsafe fn destruct(mut self, offset: usize, len: usize) {
        self.drop_range(offset, len);
        mem::forget(self);
    }

    /// # Safety
    ///
    /// This function should not be called if the locations in the range `0..len` are
    /// uninitialised.
    #[inline(always)]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        let ptr = self.array.as_ptr() as *const T;
        slice::from_raw_parts(ptr, len)
    }

    /// # Safety
    ///
    /// This function should not be called if the locations in the range `0..len` are
    /// uninitialised.
    #[inline(always)]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        let ptr = self.array.as_mut_ptr() as *mut T;
        slice::from_raw_parts_mut(ptr, len)
    }

    /// # Safety
    ///
    /// This function should not be called unless all locations in the underlying array
    /// are initialised.
    #[inline(always)]
    pub unsafe fn to_array(self) -> [T; C] {
        let src = ManuallyDrop::new(self);
        let ptr = &src.array as *const [MaybeUninit<T>; C] as _;
        ptr::read(ptr)
    }

    #[inline(always)]
    pub fn destructor(self, offset: usize, len: usize) -> Destructor<T, C> {
        Destructor {
            array: Some(self),
            offset,
            len,
        }
    }
}

impl<T, const C: usize> Drop for RawArray<T, C> {
    fn drop(&mut self) {
        panic!("drop() called instead of destruct()");
    }
}

impl<T, const C: usize> Default for RawArray<T, C> {
    #[inline(always)]
    fn default() -> Self {
        let array: [MaybeUninit<T>; C] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        Self {
            array
        }
    }
}

pub struct Destructor<T, const C: usize> {
    pub(crate) array: Option<RawArray<T, C>>,
    pub(crate) offset: usize,
    pub(crate) len: usize
}

impl<T, const C: usize> Deref for Destructor<T, C> {
    type Target = RawArray<T, C>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.array.as_ref().unwrap()
    }
}

impl<T, const C: usize> DerefMut for Destructor<T, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.array.as_mut().unwrap()
    }
}

impl<T, const C: usize> Drop for Destructor<T, C> {
    #[inline(always)]
    fn drop(&mut self) {
        if let Some(array) = self.array.take() {
            unsafe {
                array.destruct(self.offset, self.len);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod dropper {
    //! Testing of destructors.

    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Dropper(pub Rc<RefCell<usize>>);

    impl Drop for Dropper {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::stack_vec::raw_array::dropper::Dropper;

    use super::*;

    #[test]
    fn empty() {
        let array = RawArray::<String, 4>::default();
        unsafe {
            array.destruct(0, 0);
        }
    }

    #[test]
    fn empty_zero_length() {
        let array = RawArray::<String, 0>::default();
        unsafe {
            array.destruct(0, 0);
        }
    }

    #[test]
    fn empty_zst() {
        let array = RawArray::<(), 4>::default();
        unsafe {
            array.destruct(0, 0);
        }
    }

    #[test]
    fn empty_zero_length_zst() {
        let array = RawArray::<(), 0>::default();
        unsafe {
            array.destruct(0, 0);
        }
    }

    #[test]
    #[should_panic(expected = "drop() called instead of destruct()")]
    fn drop_panics() {
        let _ = RawArray::<(), 4>::default();
    }

    #[test]
    fn read_write_static_ref() {
        let mut array = RawArray::<&str, 4>::default();
        unsafe {
            array.set_and_forget(0, "zero");
            array.set_and_forget(1, "one");
            assert_eq!(&["zero", "one"], array.as_slice(2));
            assert_eq!(&["zero", "one"], array.as_mut_slice(2));

            assert_eq!("zero", *array.get(0));
            assert_eq!("one", *array.get(1));

            let slice = array.as_mut_slice(2);
            slice[0] = "0";
            slice[1] = "1";
            assert_eq!(&["0", "1"], array.as_slice(2));
            array.destruct(0, 2);
        }
    }

    #[test]
    fn read_write_owned() {
        let mut array = RawArray::<String, 4>::default();
        unsafe {
            array.set_and_forget(0, String::from("zero"));
            array.set_and_forget(1, String::from("one"));
            assert_eq!(&["zero", "one"], array.as_slice(2));
            assert_eq!(&["zero", "one"], array.as_mut_slice(2));

            assert_eq!("zero", *array.get(0));
            assert_eq!("one", *array.get(1));

            let slice = array.as_mut_slice(2);
            slice[0] = String::from("0");
            slice[1] = String::from("1");
            assert_eq!(&["0", "1"], array.as_slice(2));

            array.destruct(0, 2);
        }
    }

    #[test]
    fn read_write_zst() {
        let mut array = RawArray::<(), 4>::default();
        unsafe {
            array.set_and_forget(0, ());
            array.set_and_forget(1, ());
            assert_eq!(&[(), ()], array.as_slice(2));
            assert_eq!(&[(), ()], array.as_mut_slice(2));

            assert_eq!((), *array.get(0));
            assert_eq!((), *array.get(1));

            let slice = array.as_mut_slice(2);
            slice[0] = ();
            slice[1] = ();
            assert_eq!(&[(), ()], array.as_slice(2));

            array.destruct(0, 2);
        }
    }

    #[test]
    fn destruct_calls_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            array.destruct(0, 2);
            assert_eq!(2, *drop_count.borrow());
        }
    }

    #[test]
    fn take_calls_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            array.take(1);
            assert_eq!(1, *drop_count.borrow());
            array.destruct(0, 1);
            assert_eq!(2, *drop_count.borrow());
        }
    }

    #[test]
    fn replace_via_mut_ref_calls_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            let reference = array.get_mut(1);
            assert_eq!(0, *drop_count.borrow());

            *reference = Dropper(Rc::clone(&drop_count)); // replacing should call drop()
            assert_eq!(1, *drop_count.borrow());
            array.destruct(0, 2);
            assert_eq!(3, *drop_count.borrow());
        }
    }

    #[test]
    fn replace_via_mut_slice_calls_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            let slice = array.as_mut_slice(2);
            assert_eq!(0, *drop_count.borrow());

            slice[1] = Dropper(Rc::clone(&drop_count)); // replacing should call drop()
            assert_eq!(1, *drop_count.borrow());
            array.destruct(0, 2);
            assert_eq!(3, *drop_count.borrow());
        }
    }

    #[test]
    fn set_and_forget_does_not_call_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            array.destruct(0, 2);
            assert_eq!(2, *drop_count.borrow());
        }
    }

    #[test]
    fn to_array_moves_items() {
        let mut raw_array = RawArray::<String, 2>::default();
        unsafe {
            raw_array.set_and_forget(0, String::from("zero"));
            raw_array.set_and_forget(1, String::from("one"));

            let array = raw_array.to_array();
            assert_eq!([String::from("zero"), String::from("one")], array);
        }
    }

    #[test]
    fn to_array_moves_items_then_drops() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut raw_array = RawArray::<Dropper, 2>::default();
        unsafe {
            raw_array.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            raw_array.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());

            let array = raw_array.to_array();
            assert_eq!(0, *drop_count.borrow());
            assert_eq!(2, array.len());

            drop(array);
            assert_eq!(2, *drop_count.borrow());
        }
    }

    #[test]
    fn destructor() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let array = RawArray::<Dropper, 4>::default();
        let mut d = array.destructor(0, 0);
        unsafe {
            d.set_and_forget(0, Dropper(Rc::clone(&drop_count)));
            d.set_and_forget(1, Dropper(Rc::clone(&drop_count)));
            d.set_and_forget(2, Dropper(Rc::clone(&drop_count)));
        }
        assert_eq!(0, *drop_count.borrow());
        unsafe {
            d.take(0);
        }
        assert_eq!(1, *drop_count.borrow());
        d.offset = 1;
        d.len = 2;
        drop(d);
        assert_eq!(3, *drop_count.borrow());
    }
}
