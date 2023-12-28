use bincode::enc::Encoder;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::stack_vec::raw_array::{Destructor, Explicit, RawArray};
use bincode::de::Decoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use thiserror::Error;

pub mod raw_array;

#[macro_export]
macro_rules! sv {
    () => (
        $crate::__rust_force_expr!($crate::stack_vec::StackVec::default())
    );
    ($elem:expr; $n:expr) => (
        $crate::__rust_force_expr!($crate::stack_vec::__macro_support::sv_repeat($elem, $n))
    );
    ($($elem:expr),+ $(,)?) => {
        {
            let mut sv = $crate::stack_vec::StackVec::default();
            $(
                $crate::__rust_force_expr!(sv.push($elem));
            )*
            sv
        }
    };
}

#[macro_export]
macro_rules! __rust_force_expr {
    ($e:expr) => {
        $e
    };
}

pub mod __macro_support {
    use crate::stack_vec::StackVec;

    pub fn sv_repeat<T: Clone, const C: usize>(value: T, times: usize) -> StackVec<T, C> {
        let mut sv = StackVec::default();
        sv.repeat(value, times);
        sv
    }
}

pub struct StackVec<T, const C: usize> {
    len: usize,
    array: Explicit<RawArray<T, C>>,
    // array: [Option<T>; C]
}
impl<T, const C: usize> StackVec<T, C> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.try_push(value).unwrap_or_else(|err| panic!("{}", err))
    }

    #[inline(always)]
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityExceeded> {
        if self.len < C {
            unsafe {
                self.array.as_mut().set_and_forget(self.len, value);
            }
            self.len += 1;
            Ok(())
        } else {
            Err(CapacityExceeded { target_capacity: C })
        }
    }

    #[inline(always)]
    pub fn repeat(&mut self, value: T, times: usize)
    where
        T: Clone,
    {
        self.try_repeat(value, times)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    #[inline(always)]
    pub fn try_repeat(&mut self, value: T, times: usize) -> Result<(), CapacityExceeded>
    where
        T: Clone,
    {
        for _ in 1..times {
            self.try_push(value.clone())?;
        }
        if times > 0 {
            self.try_push(value)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn iter(&self) -> Iter<T, C> {
        Iter { sv: self, pos: 0 }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe {
            self.array.as_mut().drop_range(0, self.len);
        }
        self.len = 0;
    }

    #[inline(always)]
    pub fn fill(&mut self, value: &T) where T: Clone {
        self.clear();
        (0..C).for_each(|_| self.push(value.clone()));
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        self
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }
}

impl<T: PartialEq, const C: usize> PartialEq for StackVec<T, C> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        let self_slice = &**self;
        let other_slice = &**other;
        self_slice == other_slice
        // if self.len != other.len {
        //     return false;
        // }
        //
        // for index in 0..self.len {
        //     let self_item = unsafe { self.array.as_ref().get(index) };
        //     let other_item = unsafe { other.array.as_ref().get(index) };
        //     if self_item != other_item {
        //         return false;
        //     }
        // }
        //
        // true
    }
}
impl<T: Eq, const C: usize> Eq for StackVec<T, C> {}

impl<T: Clone, const C: usize> Clone for StackVec<T, C> {
    #[inline(always)]
    fn clone(&self) -> Self {
        let mut clone = RawArray::default();
        for i in 0..self.len {
            unsafe {
                clone.set_and_forget(i, self.array.as_ref().get(i).clone());
            }
        }
        Self {
            array: Explicit::Some(clone),
            ..*self
        }
    }
}

impl<T: Hash, const C: usize> Hash for StackVec<T, C> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let slice = &**self;
        slice.hash(state);
        // for index in 0..self.len {
        //     let item = unsafe { self.array.as_ref().get(index) };
        //     item.hash(state);
        // }
    }
}

impl<T: Debug, const C: usize> Debug for StackVec<T, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut iter = self.iter();
        if let Some(item) = iter.next() {
            write!(f, "{item:?}")?;
        }
        for item in iter {
            write!(f, ", {item:?}")?;
        }
        write!(f, "]")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("exceeds capacity ({target_capacity})")]
pub struct CapacityExceeded {
    target_capacity: usize,
}

impl<T, const B: usize, const C: usize> TryFrom<[T; B]> for StackVec<T, C> {
    type Error = CapacityExceeded;

    #[inline]
    fn try_from(source: [T; B]) -> Result<Self, Self::Error> {
        let mut sv = StackVec::default();
        for item in source {
            sv.try_push(item)?;
        }
        Ok(sv)
    }
}

impl<T: Clone, const C: usize> TryFrom<&[T]> for StackVec<T, C> {
    type Error = CapacityExceeded;

    #[inline]
    fn try_from(source: &[T]) -> Result<Self, Self::Error> {
        let mut sv = StackVec::default();
        for item in source {
            sv.try_push(item.clone())?;
        }
        Ok(sv)
    }
}

impl<T, const C: usize> FromIterator<T> for StackVec<T, C> {
    #[inline]
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let mut sv = StackVec::default();
        for item in iter {
            sv.push(item);
        }
        sv
    }
}

impl<T, const C: usize> Default for StackVec<T, C> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            len: 0,
            array: Explicit::Some(RawArray::default()),
        }
    }
}

impl<T, const C: usize> Index<usize> for StackVec<T, C> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len {
            panic!(
                "index out of bounds: the len is {} but the index is {index}",
                self.len
            );
        }
        unsafe { self.array.as_ref().get(index) }
    }
}

impl<T, const C: usize> IndexMut<usize> for StackVec<T, C> {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len {
            panic!(
                "index out of bounds: the len is {} but the index is {index}",
                self.len
            );
        }
        unsafe { self.array.as_mut().get_mut(index) }
    }
}

impl<T, const C: usize> Deref for StackVec<T, C> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.array.as_ref().as_slice(self.len) }
    }
}

impl<T, const C: usize> DerefMut for StackVec<T, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.array.as_mut().as_mut_slice(self.len) }
    }
}

pub struct Iter<'a, T, const C: usize> {
    sv: &'a StackVec<T, C>,
    pos: usize,
}

impl<'a, T, const C: usize> Iterator for Iter<'a, T, C> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.sv.len {
            let next = unsafe { self.sv.array.as_ref().get(self.pos) };
            self.pos += 1;
            Some(next)
        } else {
            None
        }
    }
}

impl<'a, T, const C: usize> IntoIterator for &'a StackVec<T, C> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T, C>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct IntoIter<T, const C: usize> {
    destructor: Destructor<T, C>,
    pos: usize,
    lim: usize,
}

impl<T, const C: usize> Iterator for IntoIter<T, C> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.lim {
            let next = unsafe { self.destructor.take(self.pos) };
            self.pos += 1;
            self.destructor.offset += 1;
            self.destructor.len -= 1;
            Some(next)
        } else {
            None
        }
    }
}

impl<T, const C: usize> IntoIterator for StackVec<T, C> {
    type Item = T;
    type IntoIter = IntoIter<T, C>;

    #[inline(always)]
    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter {
            destructor: Destructor {
                array: self.array.take(),
                offset: 0,
                len: self.len,
            },
            pos: 0,
            lim: self.len,
        }
    }
}

impl<T, const C: usize> Drop for StackVec<T, C> {
    #[inline(always)]
    fn drop(&mut self) {
        Destructor {
            array: self.array.take(),
            offset: 0,
            len: self.len,
        };
    }
}

impl<T: Encode, const C: usize> Encode for StackVec<T, C> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.len.encode(encoder)?;
        for item in self {
            item.encode(encoder)?;
        }
        Ok(())
    }
}

impl<T: Decode, const C: usize> Decode for StackVec<T, C> {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let mut sv = StackVec::default();
        let len = usize::decode(decoder)?;
        for _ in 0..len {
            let item = T::decode(decoder)?;
            sv.try_push(item)
                .map_err(|_| DecodeError::ArrayLengthMismatch {
                    required: C,
                    found: len,
                })?;
        }
        Ok(sv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack_vec::raw_array::dropper::Dropper;
    use std::cell::RefCell;
    use std::panic;
    use std::panic::AssertUnwindSafe;
    use std::rc::Rc;

    #[test]
    fn init() {
        let sv = StackVec::<String, 4>::default();
        assert!(sv.is_empty());
        assert_eq!(0, sv.len());
        assert_eq!(None, sv.iter().next());
        assert_eq!(None, sv.into_iter().next());
    }

    #[test]
    fn init_zst() {
        let sv = StackVec::<(), 4>::default();
        assert!(sv.is_empty());
        assert_eq!(0, sv.len());
        assert_eq!(None, sv.iter().next());
        assert_eq!(None, sv.into_iter().next());
    }

    #[test]
    fn init_zero_length_zst() {
        let sv = StackVec::<(), 0>::default();
        assert!(sv.is_empty());
        assert_eq!(0, sv.len());
        assert_eq!(None, sv.iter().next());
        assert_eq!(None, sv.into_iter().next());
    }

    #[test]
    fn clone() {
        {
            let sv = StackVec::<(), 4>::default();
            let clone = sv.clone();
            assert!(clone.is_empty());
        }
        {
            let sv: StackVec<_, 2> = sv!["zero"];
            let clone = sv.clone();
            assert_eq!(vec!["zero"], clone.into_iter().collect::<Vec<_>>());
        }
        {
            let sv: StackVec<_, 2> = sv![String::from("zero"), String::from("one")];
            let clone = sv.clone();
            assert_eq!(vec!["zero", "one"], clone.into_iter().collect::<Vec<_>>());
        }
        {
            let sv: StackVec<_, 2> = sv![(), ()];
            let clone = sv.clone();
            assert_eq!(vec![(), ()], clone.into_iter().collect::<Vec<_>>());
        }
    }

    #[test]
    fn clone_eventually_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let sv: StackVec<_, 2> = sv![
            Dropper(Rc::clone(&drop_count)),
            Dropper(Rc::clone(&drop_count))
        ];
        let clone = sv.clone();
        assert_eq!(2, clone.iter().count());
        assert_eq!(0, *drop_count.borrow());
        drop(clone);
        assert_eq!(2, *drop_count.borrow());
        drop(sv);
        assert_eq!(4, *drop_count.borrow());
    }

    #[test]
    fn eq() {
        let a: StackVec<&str, 2> = sv![];
        let b: StackVec<_, 2> = sv!["zero"];
        let c1: StackVec<_, 2> = sv!["zero", "one"];
        let c2: StackVec<_, 2> = sv!["zero", "one"];
        assert_ne!(a, b);
        assert_ne!(b, c1);
        assert_eq!(a, a);
        assert_eq!(b, b);
        assert_eq!(c1, c1);
        assert_eq!(c1, c2);
    }

    #[test]
    fn macro_and_debug() {
        {
            let sv: StackVec<(), 0> = sv![];
            assert_eq!("[]", format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero"];
            assert_eq!(r#"["zero"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv![String::from("zero")];
            assert_eq!(r#"["zero"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero",];
            assert_eq!(r#"["zero"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero", "one"];
            assert_eq!(r#"["zero", "one"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero", "one",];
            assert_eq!(r#"["zero", "one"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 3> = sv!["zero", "one", "two"];
            assert_eq!(r#"["zero", "one", "two"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero"; 0];
            assert_eq!(r#"[]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero"; 1];
            assert_eq!(r#"["zero"]"#, format!("{sv:?}"));
        }
        {
            let sv: StackVec<_, 2> = sv!["zero"; 2];
            assert_eq!(r#"["zero", "zero"]"#, format!("{sv:?}"));
        }
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn macro_exceeds_capacity_elements() {
        let _: StackVec<_, 2> = sv![
            String::from("zero"),
            String::from("one"),
            String::from("two")
        ];
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn macro_exceeds_capacity_repeat() {
        let _: StackVec<_, 2> = sv![String::from("zero"); 3];
    }

    #[test]
    fn push_within_capacity() {
        let mut sv = StackVec::<_, 4>::default();
        sv.push("zero");
        assert!(!sv.is_empty());
        assert_eq!(1, sv.len());
        sv.push("one");
        sv.push("two");
        sv.push("three");
        assert_eq!(4, sv.len());
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn push_with_overflow_static_ref() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push("zero");
        sv.push("one");
        sv.push("two");
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn push_with_overflow_owned() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push(String::from("zero"));
        sv.push(String::from("one"));
        sv.push(String::from("two"));
    }

    #[test]
    #[cfg(panic = "unwind")]
    fn push_with_overflow_will_drop() {
        let drop_count = AssertUnwindSafe(Rc::new(RefCell::new(0_usize)));
        let result = panic::catch_unwind(|| {
            let mut sv = StackVec::<_, 2>::default();
            sv.push(Dropper(Rc::clone(&drop_count)));
            sv.push(Dropper(Rc::clone(&drop_count)));
            sv.push(Dropper(Rc::clone(&drop_count)));
        });
        assert!(result.is_err());
        assert_eq!(3, *drop_count.borrow());
    }

    #[test]
    fn try_push() {
        let mut sv = StackVec::<_, 2>::default();
        assert!(sv.try_push("zero").is_ok());
        assert!(sv.try_push("one").is_ok());
        assert_eq!(2, sv.len());
        assert!(sv.try_push("two").is_err());
        assert_eq!(2, sv.len());
    }

    #[test]
    fn iter() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push("zero");
        sv.push("one");
        let mut iter = sv.iter();
        assert_eq!(Some(&"zero"), iter.next());
        assert_eq!(Some(&"one"), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn iter_does_not_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv = StackVec::<_, 2>::default();
        sv.push(Dropper(Rc::clone(&drop_count)));
        sv.push(Dropper(Rc::clone(&drop_count)));
        let iter = sv.iter();
        assert_eq!(2, iter.count());
        assert_eq!(0, *drop_count.borrow());
    }

    #[test]
    fn iter_ref() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push("zero");
        sv.push("one");
        let mut vec = Vec::with_capacity(2);
        for &item in &sv {
            vec.push(item);
        }
        assert_eq!(vec!["zero", "one"], vec);
    }

    #[test]
    fn into_iter() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push("zero");
        sv.push("one");
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn into_iter_consume_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv = StackVec::<_, 2>::default();
        sv.push(Dropper(Rc::clone(&drop_count)));
        sv.push(Dropper(Rc::clone(&drop_count)));
        assert_eq!(0, *drop_count.borrow());
        let into_iter = sv.into_iter();
        assert_eq!(0, *drop_count.borrow());
        assert_eq!(2, into_iter.count());
        assert_eq!(2, *drop_count.borrow());
    }

    #[test]
    fn into_iter_partial_consume_will_drop_remaining() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv = StackVec::<_, 4>::default();
        sv.push(Dropper(Rc::clone(&drop_count)));
        sv.push(Dropper(Rc::clone(&drop_count)));
        sv.push(Dropper(Rc::clone(&drop_count)));
        assert_eq!(0, *drop_count.borrow());
        let mut into_iter = sv.into_iter();
        assert_eq!(0, *drop_count.borrow());
        assert!(into_iter.next().is_some());
        assert_eq!(1, *drop_count.borrow());
        drop(into_iter);
        assert_eq!(3, *drop_count.borrow());
    }

    #[test]
    fn as_slice() {
        let sv: StackVec<_, 2> = sv![String::from("zero"), String::from("one")];
        assert_eq!(&[String::from("zero"), String::from("one")], sv.as_slice());
    }

    #[test]
    fn as_mut_slice() {
        let mut sv: StackVec<_, 2> = sv![String::from("zero"), String::from("one")];
        let slice = &mut *sv;
        slice[0] = String::from("0");
        assert_eq!(&[String::from("0"), String::from("one")], sv.as_mut_slice());
    }

    #[test]
    fn as_mut_slice_replace_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv: StackVec<_, 2> = sv![
            Dropper(Rc::clone(&drop_count)),
            Dropper(Rc::clone(&drop_count))
        ];
        let slice = &mut *sv;
        assert_eq!(0, *drop_count.borrow());
        slice[0] = Dropper(Rc::clone(&drop_count));
        assert_eq!(1, *drop_count.borrow());
        assert_eq!(2, sv.len());
        drop(sv);
        assert_eq!(3, *drop_count.borrow());
    }

    #[test]
    fn from_array() {
        let sv = StackVec::<_, 2>::try_from(["zero", "one"]).unwrap();
        assert_eq!(2, sv.len());
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn from_slice() {
        let slice = ["zero", "one"].as_slice();
        let sv = StackVec::<&str, 2>::try_from(slice).unwrap();
        assert_eq!(2, sv.len());
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn from_array_overflow() {
        let result = StackVec::<_, 2>::try_from(["zero", "one", "two"]);
        assert_eq!(CapacityExceeded { target_capacity: 2 }, result.unwrap_err());
    }

    #[test]
    fn from_iterator() {
        let sv = (0..3).collect::<StackVec<_, 3>>();
        assert_eq!(3, sv.len);
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (3)")]
    fn from_iterator_overflow() {
        let _ = (0..4).collect::<StackVec<_, 3>>();
    }

    #[test]
    fn index() {
        let sv: StackVec<_, 2> = sv!["zero", "one"];
        assert_eq!("zero", sv[0]);
        assert_eq!("one", sv[1]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
    fn index_overflow() {
        let sv: StackVec<_, 2> = sv!["0", "1"];
        let _ = sv[2];
    }

    #[test]
    #[cfg(panic = "unwind")]
    fn index_overflow_will_drop() {
        let drop_count = AssertUnwindSafe(Rc::new(RefCell::new(0_usize)));
        let result = panic::catch_unwind(|| {
            let sv: StackVec<_, 2> = sv![
                Dropper(Rc::clone(&drop_count)),
                Dropper(Rc::clone(&drop_count))
            ];
            let _ = sv[2];
        });
        assert!(result.is_err());
        assert_eq!(2, *drop_count.borrow());
    }

    #[test]
    fn index_mut() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv[0] = "zero";
        sv[1] = "one";
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn index_mut_replace_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv: StackVec<_, 2> = sv![
            Dropper(Rc::clone(&drop_count)),
            Dropper(Rc::clone(&drop_count))
        ];
        assert_eq!(0, *drop_count.borrow());
        sv[0] = Dropper(Rc::clone(&drop_count));
        assert_eq!(1, *drop_count.borrow());
        sv[1] = Dropper(Rc::clone(&drop_count));
        assert_eq!(2, *drop_count.borrow());
        assert_eq!(2, sv.len());
        drop(sv);
        assert_eq!(4, *drop_count.borrow());
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
    fn index_mut_overflow() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv[2] = "two";
    }

    #[test]
    fn clear_static_ref() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv.clear();
        assert!(sv.is_empty());
        assert_eq!(Vec::<&str>::new(), sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn clear_owned() {
        let mut sv: StackVec<_, 2> = sv![String::from("0"), String::from("1")];
        sv.clear();
        assert!(sv.is_empty());
        assert_eq!(Vec::<&str>::new(), sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn clear_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let mut sv: StackVec<_, 2> = sv![
            Dropper(Rc::clone(&drop_count)),
            Dropper(Rc::clone(&drop_count))
        ];
        assert_eq!(0, *drop_count.borrow());
        sv.clear();
        assert_eq!(2, *drop_count.borrow());
        assert!(sv.is_empty());
        drop(sv);
        assert_eq!(2, *drop_count.borrow());
    }

    #[test]
    fn fill_owned() {
        let mut sv: StackVec<_, 3> = sv![String::from("0"), String::from("1")];
        sv.fill(&String::from("9"));
        assert_eq!(vec![String::from("9"), String::from("9"), String::from("9")], sv.to_vec());
    }

    #[test]
    fn fill_will_drop() {
        let drop_count = Rc::new(RefCell::new(0_usize));
        let dropper = Dropper(Rc::clone(&drop_count));
        let mut sv: StackVec<_, 3> = sv![dropper.clone(), dropper.clone()];
        assert_eq!(0, *drop_count.borrow());

        sv.fill(&dropper);
        assert_eq!(3, sv.len());
        assert_eq!(2, *drop_count.borrow());

        drop(sv);
        assert_eq!(5, *drop_count.borrow());
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn sv_overflow() {
        let _: StackVec<_, 2> = sv!["0", "1", "2"];
    }

    #[test]
    #[cfg(panic = "unwind")]
    fn sv_overflow_will_drop() {
        let drop_count = AssertUnwindSafe(Rc::new(RefCell::new(0_usize)));
        let result = panic::catch_unwind(|| {
            let _: StackVec<_, 2> = sv![
                Dropper(Rc::clone(&drop_count)),
                Dropper(Rc::clone(&drop_count)),
                Dropper(Rc::clone(&drop_count))
            ];
        });
        assert!(result.is_err());
        assert_eq!(3, *drop_count.borrow());
    }

    #[test]
    fn encode_then_decode() {
        let input: StackVec<_, 4> = sv![String::from("zero"), String::from("one")];
        let bytes = bincode::encode_to_vec(&input, bincode::config::standard()).unwrap();
        let (output, _) = bincode::decode_from_slice::<StackVec<String, 4>, _>(
            &bytes,
            bincode::config::standard(),
        )
        .unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn encode_then_decode_into_larger() {
        let input: StackVec<_, 2> = sv![String::from("zero"), String::from("one")];
        let bytes = bincode::encode_to_vec(&input, bincode::config::standard()).unwrap();
        let (output, _) = bincode::decode_from_slice::<StackVec<String, 3>, _>(
            &bytes,
            bincode::config::standard(),
        )
        .unwrap();
        assert_eq!(
            input.into_iter().collect::<Vec<_>>(),
            output.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn encode_then_decode_into_smaller_within_capacity() {
        let input: StackVec<_, 3> = sv![String::from("zero"), String::from("one")];
        let bytes = bincode::encode_to_vec(&input, bincode::config::standard()).unwrap();
        let (output, _) = bincode::decode_from_slice::<StackVec<String, 2>, _>(
            &bytes,
            bincode::config::standard(),
        )
        .unwrap();
        assert_eq!(
            input.into_iter().collect::<Vec<_>>(),
            output.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn encode_then_decode_into_smaller_overflow() {
        let input: StackVec<_, 2> = sv![String::from("zero"), String::from("one")];
        let bytes = bincode::encode_to_vec(&input, bincode::config::standard()).unwrap();
        let result = bincode::decode_from_slice::<StackVec<String, 1>, _>(
            &bytes,
            bincode::config::standard(),
        );
        match result {
            Err(DecodeError::ArrayLengthMismatch { required, found }) => {
                assert_eq!(1, required);
                assert_eq!(2, found);
            }
            _ => panic!("expecting an ArrayLengthMismatch error"),
        }
    }
}
