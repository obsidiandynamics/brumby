use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut};
use thiserror::Error;

#[derive(Clone, Eq)]
pub struct StackVec<T, const C: usize> {
    len: usize,
    array: [Option<T>; C],
}
impl<T, const C: usize> StackVec<T, C> {
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        self.try_push(value).unwrap_or_else(|err| panic!("{}", err))
    }

    #[inline]
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityExceeded> {
        if self.len < C {
            self.array[self.len] = Some(value);
            self.len += 1;
            Ok(())
        } else {
            Err(CapacityExceeded { target_capacity: C })
        }
    }

    #[inline]
    pub fn push_repeat(&mut self, value: T, times: usize)
    where
        T: Clone,
    {
        self.try_push_repeat(value, times)
            .unwrap_or_else(|err| panic!("{}", err))
    }

    #[inline]
    pub fn try_push_repeat(&mut self, value: T, times: usize) -> Result<(), CapacityExceeded>
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

    pub fn iter(&self) -> Iter<T, C> {
        Iter { sv: self, pos: 0 }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.array.fill_with(|| None);
        self.len = 0;
    }
}

impl<T: PartialEq, const C: usize> PartialEq for StackVec<T, C> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }

        for index in 0..self.len {
            let self_item = &self.array[index];
            let other_item = &other.array[index];
            if self_item != other_item {
                return false;
            }
        }

        true
    }
}

impl<T: Hash, const C: usize> Hash for StackVec<T, C> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        for index in 0..self.len {
            let item = &self.array[index];
            item.hash(state);
        }
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

    fn try_from(source: [T; B]) -> Result<Self, Self::Error> {
        if B > C {
            return Err(CapacityExceeded { target_capacity: C });
        }

        let mut array: [Option<T>; C] = std::array::from_fn(|_| None);
        for (index, item) in source.into_iter().enumerate() {
            array[index] = Some(item);
        }
        Ok(Self { len: B, array })
    }
}

// pub fn si<J, T, const C: usize>(source: J) -> StackVec<T, C> where J: IntoIterator<Item = T> {
//     let mut sv = StackVec::default();
//     for item in source {
//         sv.push(item);
//     }
//     sv
// }

pub mod __macro_support {
    use crate::stack_vec::StackVec;

    pub fn sv_repeat<T: Clone, const C: usize>(value: T, times: usize) -> StackVec<T, C> {
        let mut sv = StackVec::default();
        sv.push_repeat(value, times);
        sv
    }
}

#[macro_export]
macro_rules! sv {
    () => (
        $crate::__rust_force_expr!($crate::stack_vec::StackVec::default())
    );
    ($elem:expr; $n:expr) => (
        $crate::__rust_force_expr!($crate::stack_vec::__macro_support::sv_repeat($elem, $n))
    );
    ($($elem:expr), *) => {
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

impl<T, const C: usize> Default for StackVec<T, C> {
    #[inline]
    fn default() -> Self {
        Self {
            len: 0,
            array: std::array::from_fn(|_| None),
        }
    }
}

impl<T, const C: usize> Index<usize> for StackVec<T, C> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len {
            panic!(
                "index out of bounds: the len is {} but the index is {index}",
                self.len
            );
        }
        self.array[index].as_ref().unwrap()
    }
}

impl<T, const C: usize> IndexMut<usize> for StackVec<T, C> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len {
            panic!(
                "index out of bounds: the len is {} but the index is {index}",
                self.len
            );
        }
        self.array[index].as_mut().unwrap()
    }
}

pub struct Iter<'a, T, const C: usize> {
    sv: &'a StackVec<T, C>,
    pos: usize,
}

impl<'a, T, const C: usize> Iterator for Iter<'a, T, C> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.sv.len {
            let next = self.sv.array[self.pos].as_ref();
            self.pos += 1;
            next
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
    sv: StackVec<T, C>,
    pos: usize,
}

impl<T, const C: usize> Iterator for IntoIter<T, C> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.sv.len {
            let next = self.sv.array[self.pos].take();
            self.pos += 1;
            next
        } else {
            None
        }
    }
}

impl<T, const C: usize> IntoIterator for StackVec<T, C> {
    type Item = T;
    type IntoIter = IntoIter<T, C>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { sv: self, pos: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init() {
        let sv = StackVec::<(), 4>::default();
        assert!(sv.is_empty());
        assert_eq!(0, sv.len());
        assert_eq!(None, sv.iter().next());
        assert_eq!(None, sv.into_iter().next());
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
            let sv: StackVec<_, 2> = sv!["zero", "one"];
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
    fn push_with_overflow() {
        let mut sv = StackVec::<_, 2>::default();
        sv.push("zero");
        sv.push("one");
        sv.push("two");
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
    fn from_array() {
        let sv = StackVec::<_, 2>::try_from(["zero", "one"]).unwrap();
        assert_eq!(2, sv.len());
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn from_array_overflow() {
        let result = StackVec::<_, 2>::try_from(["zero", "one", "two"]);
        assert_eq!(CapacityExceeded { target_capacity: 2 }, result.unwrap_err());
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
    fn index_mut() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv[0] = "zero";
        sv[1] = "one";
        assert_eq!(vec!["zero", "one"], sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
    fn index_mut_overflow() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv[2] = "two";
    }

    #[test]
    fn clear() {
        let mut sv: StackVec<_, 2> = sv!["0", "1"];
        sv.clear();
        assert!(sv.is_empty());
        assert_eq!(Vec::<&str>::new(), sv.into_iter().collect::<Vec<_>>());
    }

    #[test]
    #[should_panic(expected = "exceeds capacity (2)")]
    fn sv_overflow() {
        let _: StackVec<_, 2> = sv!["0", "1", "2"];
    }
}
