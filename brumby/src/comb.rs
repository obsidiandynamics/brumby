//! Combinatorics.

#[inline(always)]
pub fn pick(cardinalities: &[usize], permutation: u64, ordinals: &mut [usize]) {
    let mut residual = permutation;
    for (index, &cardinality) in cardinalities.iter().enumerate() {
        let cardinality = cardinality as u64;
        let (quotient, remainder) = (residual / cardinality, residual % cardinality);
        residual = quotient;
        ordinals[index] = remainder as usize;
    }
}

#[inline]
pub fn count_permutations(cardinalities: &[usize]) -> u64 {
    cardinalities.iter().fold(1u64, |acc, &num| acc * num as u64)
}

pub struct Permuter<'a> {
    cardinalities: &'a [usize],
    permutations: u64,
}
impl<'a> Permuter<'a> {
    pub fn new(cardinalities: &'a [usize]) -> Self {
        let permutations = count_permutations(cardinalities);
        Self {
            cardinalities,
            permutations,
        }
    }
}

impl<'a> IntoIterator for Permuter<'a> {
    type Item = Vec<usize>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            permuter: self,
            permutation: 0,
        }
    }
}

pub struct Iter<'a> {
    permuter: Permuter<'a>,
    permutation: u64,
}
impl<'a> Iterator for Iter<'a> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.permutation != self.permuter.permutations {
            let mut ordinals = vec![0; self.permuter.cardinalities.len()];
            pick(
                self.permuter.cardinalities,
                self.permutation,
                &mut ordinals,
            );
            self.permutation += 1;
            Some(ordinals)
        } else {
            None
        }
    }
}

#[inline(always)]
pub fn is_unique_quadratic(elements: &[usize]) -> bool {
    for (index, element) in elements.iter().enumerate() {
        for other in &elements[index + 1..] {
            if element == other {
                return false;
            }
        }
    }
    true
}

#[inline(always)]
pub fn is_unique_linear(elements: &[usize], bitmap: &mut [bool]) -> bool {
    bitmap.fill(false);
    for &element in elements {
        if bitmap[element] {
            return false;
        }
        bitmap[element] = true;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick() {
        let cardinalities = &[2, 3, 4];
        let mut outputs = vec![];
        let permutations = count_permutations(cardinalities);
        assert_eq!(24, permutations);
        for permutation in 0..permutations {
            let mut ordinals = [0; 3];
            pick(cardinalities, permutation, &mut ordinals);
            outputs.push(ordinals.to_vec());
            println!("ordinals: {ordinals:?}");
        }
        let expected_outputs = vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 2, 0],
            [1, 2, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
            [0, 2, 1],
            [1, 2, 1],
            [0, 0, 2],
            [1, 0, 2],
            [0, 1, 2],
            [1, 1, 2],
            [0, 2, 2],
            [1, 2, 2],
            [0, 0, 3],
            [1, 0, 3],
            [0, 1, 3],
            [1, 1, 3],
            [0, 2, 3],
            [1, 2, 3],
        ]
        .iter()
        .map(|array| array.to_vec())
        .collect::<Vec<_>>();
        assert_eq!(expected_outputs, outputs);
    }

    #[test]
    fn iterator() {
        let permuter = Permuter::new(&[2, 3, 4]);
        let outputs = permuter.into_iter().collect::<Vec<_>>();
        let expected_outputs = vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 2, 0],
            [1, 2, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
            [0, 2, 1],
            [1, 2, 1],
            [0, 0, 2],
            [1, 0, 2],
            [0, 1, 2],
            [1, 1, 2],
            [0, 2, 2],
            [1, 2, 2],
            [0, 0, 3],
            [1, 0, 3],
            [0, 1, 3],
            [1, 1, 3],
            [0, 2, 3],
            [1, 2, 3],
        ]
        .iter()
        .map(|array| array.to_vec())
        .collect::<Vec<_>>();
        assert_eq!(expected_outputs, outputs);
    }

    #[test]
    fn test_is_unique_quadratic() {
        assert!(is_unique_quadratic(&[]));
        assert!(is_unique_quadratic(&[1]));
        assert!(is_unique_quadratic(&[1, 2, 3]));
        assert!(!is_unique_quadratic(&[1, 1]));
        assert!(!is_unique_quadratic(&[1, 0, 1]));
    }

    #[test]
    fn test_is_unique_linear() {
        let mut bitmap_0 = vec![false; 0];
        let mut bitmap_1 = vec![false; 1];
        let mut bitmap_2 = vec![false; 2];
        let mut bitmap_3 = vec![false; 3];

        assert!(is_unique_linear(&[], &mut bitmap_0));
        assert!(is_unique_linear(&[0], &mut bitmap_1));
        assert!(is_unique_linear(&[0, 1, 2], &mut bitmap_3));
        assert!(is_unique_linear(&[2, 1, 0], &mut bitmap_3));
        assert!(!is_unique_linear(&[0, 0], &mut bitmap_2));
        assert!(!is_unique_linear(&[1, 0, 1], &mut bitmap_3));
    }
}
