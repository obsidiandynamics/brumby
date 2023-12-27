use std::{mem, slice};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

pub struct RawArray<T, const C: usize> {
    array: [T; C]
}
impl<T, const C: usize> RawArray<T, C> {
    // #[inline]
    // pub fn new() -> Self {
    //     // unsafe {
    //     //     let ptr: *mut T = MaybeUninit::uninit().assume_init();
    //     //     let array = transmute::<_, [T; C]>(ptr);
    //     //     Self {
    //     //         array
    //     //     }
    //     // }
    //
    //     unsafe {
    //         let uninit_ptr: MaybeUninit<[T; C]> = MaybeUninit::uninit();
    //         let array = uninit_ptr.assume_init();
    //         Self {
    //             array
    //         }
    //     }
    // }

    #[inline]
    pub unsafe fn get(&self, index: usize) -> &T {
        let ptr = self.array.as_ptr();
        let pos = ptr.add(index);
        &*pos
    }

    #[inline]
    pub unsafe fn set(&mut self, index: usize, value: T) {
        let ptr = self.array.as_mut_ptr();
        let pos = ptr.add(index);
        pos.write(value);
    }

    #[inline]
    pub unsafe fn take(&mut self, index: usize) -> T {
        let ptr = self.array.as_mut_ptr();
        let pos = ptr.add(index);
        pos.read()
    }

    #[inline]
    pub unsafe fn drop_range(mut self, offset: usize, len: usize) {
        let ptr = self.array.as_mut_ptr();
        for index in offset..offset + len {
            let pos = ptr.add(index);
            let value = pos.read();
            drop(value);
        }
        mem::forget(self);
    }

    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        let ptr = self.array.as_ptr();
        slice::from_raw_parts(ptr, len)
    }

    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        let ptr = self.array.as_mut_ptr();
        slice::from_raw_parts_mut(ptr, len)
    }
}

impl<T, const C: usize> Default for RawArray<T, C> {
    #[inline]
    fn default() -> Self {
        unsafe {
            let uninit_ptr: MaybeUninit<[T; C]> = MaybeUninit::uninit();
            let array = uninit_ptr.assume_init();
            Self {
                array
            }
        }
    }
}

pub struct Destructor<T, const C: usize> {
    array: Explicit<RawArray<T, C>>,
    offset: usize,
    len: usize
}

impl<T, const C: usize> Deref for Destructor<T, C> {
    type Target = RawArray<T, C>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.array.as_ref()
    }
}

impl<T, const C: usize> DerefMut for Destructor<T, C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.array.as_mut()
    }
}

impl<T, const C: usize> Drop for Destructor<T, C> {
    #[inline]
    fn drop(&mut self) {
        let array = self.array.take();
        unsafe {
            array.drop_range(self.offset, self.len);
        }
    }
}

// #[inline]
// unsafe fn transmute<Src, Dst>(src: Src) -> Dst {
//     let dst = ptr::read(&src as *const Src as *const Dst);
//     mem::forget(src);
//     dst
// }

/// A variant of `Option` that omits the "null pointer optimisation" (NPO). By adding a third variant,
/// the variant is determined explicitly from the stored tag, rather than implicitly. This enables
/// the encapsulation of uninitialised data, which would otherwise appear as [None] under
/// NPO.
enum Explicit<T> {
    None,
    Some(T),
    __Other
}
impl<T> Explicit<T> {
    // fn is_some(&self) -> bool {
    //     matches!(self, Unoption::Some(_))
    // }

    fn as_ref(&self) -> &T {
        match self {
            Explicit::Some(value) => value,
            _ => panic!("invalid state")
        }
    }

    fn as_mut(&mut self) -> &mut T {
        match self {
            Explicit::Some(value) => value,
            _ => panic!("invalid state")
        }
    }

    fn take(&mut self) -> T {
        let mut replacement = Explicit::__Other;
        mem::swap(self, &mut replacement);
        match replacement {
            Explicit::Some(value) => value,
            _ => panic!("invalid state")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn empty() {
        let array = RawArray::<String, 4>::default();
        unsafe {
            array.drop_range(0, 0);
        }
    }

    #[test]
    fn read_write_static_ref() {
        let mut array = RawArray::<&str, 4>::default();
        unsafe {
            array.set(0, "zero");
            array.set(1, "one");
            assert_eq!(&["zero", "one"], array.as_slice(2));
            assert_eq!(&["zero", "one"], array.as_mut_slice(2));

            assert_eq!("zero", *array.get(0));
            assert_eq!("one", *array.get(1));

            let slice = array.as_mut_slice(2);
            slice[0] = "0";
            slice[1] = "1";
            assert_eq!(&["0", "1"], array.as_slice(2));
            array.drop_range(0, 2);
        }
    }

    #[test]
    fn read_write_owned() {
        let mut array = RawArray::<String, 4>::default();
        unsafe {
            array.set(0, String::from("zero"));
            array.set(1, String::from("one"));
            assert_eq!(&["zero", "one"], array.as_slice(2));
            assert_eq!(&["zero", "one"], array.as_mut_slice(2));

            assert_eq!("zero", *array.get(0));
            assert_eq!("one", *array.get(1));

            let slice = array.as_mut_slice(2);
            slice[0] = String::from("0");
            slice[1] = String::from("1");
            assert_eq!(&["0", "1"], array.as_slice(2));

            array.drop_range(0, 2);
        }
    }

    /// Testing of destructors.
    struct Dropper(Rc<RefCell<usize>>);
    impl Drop for Dropper {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }

    #[test]
    fn destroy_calls_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut array = RawArray::<Dropper, 4>::default();
        unsafe {
            array.set(0, Dropper(Rc::clone(&drop_count)));
            array.set(1, Dropper(Rc::clone(&drop_count)));
            assert_eq!(0, *drop_count.borrow());
            array.drop_range(0, 2);
            assert_eq!(2, *drop_count.borrow());
        }
    }

    #[test]
    fn destructor() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let array = RawArray::<Dropper, 4>::default();
        let mut d = Destructor {
            array: Explicit::Some(array),
            offset: 0,
            len: 0,
        };
        unsafe {
            d.set(0, Dropper(Rc::clone(&drop_count)));
            d.set(1, Dropper(Rc::clone(&drop_count)));
            d.set(2, Dropper(Rc::clone(&drop_count)));
        }
        assert_eq!(0, *drop_count.borrow());
        unsafe {
            d.take(0);
        }
        assert_eq!(1, *drop_count.borrow());
        d.offset = 1;
        d.len = 2;
        // mem::forget(d);
        drop(d);
        assert_eq!(3, *drop_count.borrow());
    }
}
