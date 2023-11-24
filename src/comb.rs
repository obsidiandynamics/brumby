//! Combinatorics.

pub fn pick(cardinalities: &[usize], combination: u64, ordinals: &mut [usize]) {
    let mut residual = combination;
    for (index, &cardinality) in cardinalities.iter().enumerate() {
        let cardinality = cardinality as u64;
        let (quotient, remainder) = (residual / cardinality, residual % cardinality);
        residual = quotient;
        ordinals[index] = remainder as usize;
    }
}

pub fn count_combinations(cardinalities: &[usize]) -> u64 {
    cardinalities.iter().product::<usize>() as u64
}

pub struct Combinator<'a> {
    cardinalities: &'a [usize],
    combinations: u64,
}
impl<'a> Combinator<'a> {
    pub fn new(cardinalities: &'a [usize]) -> Self {
        let combinations = count_combinations(cardinalities);
        Self {
            cardinalities,
            combinations,
        }
    }
}

impl<'a> IntoIterator for Combinator<'a> {
    type Item = Vec<usize>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            combinator: self,
            combination: 0,
        }
    }
}

pub struct Iter<'a> {
    combinator: Combinator<'a>,
    combination: u64,
}
impl<'a> Iterator for Iter<'a> {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.combination != self.combinator.combinations {
            let mut ordinals = vec![0; self.combinator.cardinalities.len()];
            pick(
                self.combinator.cardinalities,
                self.combination,
                &mut ordinals,
            );
            self.combination += 1;
            Some(ordinals)
        } else {
            None
        }
    }
}

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
        let combinations = count_combinations(cardinalities);
        assert_eq!(24, combinations);
        for combination in 0..combinations {
            let mut ordinals = [0; 3];
            pick(cardinalities, combination, &mut ordinals);
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
        let combinator = Combinator::new(&[2, 3, 4]);
        let outputs = combinator.into_iter().collect::<Vec<_>>();
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
