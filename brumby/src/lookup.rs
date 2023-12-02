use std::hash::Hash;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct Index<T: Eq + PartialEq + Hash> {
    item_to_index: FxHashMap<T, usize>,
    index_to_item: Vec<T>,
}
impl<T: Eq + PartialEq + Hash> Index<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let index_to_item = FxHashMap::with_capacity_and_hasher(capacity, Default::default());
        let item_to_index = Vec::with_capacity(capacity);
        Self {
            item_to_index: index_to_item,
            index_to_item: item_to_index
        }
    }

    pub fn push(&mut self, item: T) where T: Clone {
        self.item_to_index.insert(item.clone(), self.item_to_index.len());
        self.index_to_item.push(item);
    }

    pub fn item(&self, index: usize) -> Option<&T> {
        self.index_to_item.get(index)
    }

    pub fn index(&self, item: &T) -> Option<usize> {
        self.item_to_index.get(&item).map(|&index| index)
    }

    pub fn len(&self) -> usize {
        self.index_to_item.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_lookup() {
        let mut index = Index::with_capacity(3);
        assert_eq!(0, index.len());
        index.push("zero");
        index.push("one");
        assert_eq!(2, index.len());

        assert_eq!(Some(&"zero"), index.item(0));
        assert_eq!(Some(0), index.index(&"zero"));

        assert_eq!(Some(&"one"), index.item(1));
        assert_eq!(Some(1), index.index(&"one"));

        assert_eq!(None, index.item(2));
        assert_eq!(None, index.index(&"two"));
    }
}