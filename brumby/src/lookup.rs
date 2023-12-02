use rustc_hash::FxHashMap;
use std::hash::Hash;
use std::ops::Index;

#[derive(Debug)]
pub struct Lookup<T: Eq + PartialEq + Hash> {
    item_to_index: FxHashMap<T, usize>,
    index_to_item: Vec<T>,
}
impl<T: Eq + PartialEq + Hash> Lookup<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let item_to_index = FxHashMap::with_capacity_and_hasher(capacity, Default::default());
        let index_to_item = Vec::with_capacity(capacity);
        Self {
            item_to_index,
            index_to_item,
        }
    }

    pub fn push(&mut self, item: T)
    where
        T: Clone,
    {
        self.item_to_index
            .insert(item.clone(), self.item_to_index.len());
        self.index_to_item.push(item);
    }

    pub fn item_at(&self, index: usize) -> Option<&T> {
        self.index_to_item.get(index)
    }

    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.item_to_index.get(&item).map(|&index| index)
    }

    pub fn len(&self) -> usize {
        self.index_to_item.len()
    }
}

impl<T: Eq + PartialEq + Hash> Index<usize> for Lookup<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.item_at(index)
            .unwrap_or_else(|| panic!("no item at index {index}"))
    }
}

impl<T: Eq + PartialEq + Hash + Clone> From<Vec<T>> for Lookup<T> {
    fn from(index_to_item: Vec<T>) -> Self {
        let mut item_to_index =
            FxHashMap::with_capacity_and_hasher(index_to_item.len(), Default::default());
        for (index, item) in index_to_item.iter().enumerate() {
            item_to_index.insert(item.clone(), index);
        }
        Self {
            item_to_index,
            index_to_item,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_resolve() {
        let mut lookup = Lookup::with_capacity(3);
        assert_eq!(0, lookup.len());
        lookup.push("zero");
        lookup.push("one");
        assert_eq!(2, lookup.len());

        assert_eq!(Some(&"zero"), lookup.item_at(0));
        assert_eq!(Some(0), lookup.index_of(&"zero"));

        assert_eq!(Some(&"one"), lookup.item_at(1));
        assert_eq!(Some(1), lookup.index_of(&"one"));

        assert_eq!(None, lookup.item_at(2));
        assert_eq!(None, lookup.index_of(&"two"));
    }

    #[test]
    fn from_vec() {
        let lookup = Lookup::from(vec!["zero", "one"]);
        assert_eq!(Some(&"zero"), lookup.item_at(0));
        assert_eq!(Some(1), lookup.index_of(&"one"));
        assert_eq!(2, lookup.len());
    }

    #[test]
    #[should_panic(expected = "no item at index 2")]
    fn no_item_at_index() {
        let lookup = Lookup::from(vec!["zero", "one"]);
        lookup[2];
    }
}
