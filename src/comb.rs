//! Combinatorics.

pub fn vectorize(cardinalities: &[usize], combination: u64, ordinals: &mut [usize]) {
    let mut residual = combination;
    for (index, &cardinality) in cardinalities.iter().enumerate() {
        let cardinality = cardinality as u64;
        let (quotient, remainder) = (residual / cardinality, residual % cardinality);
        residual = quotient;
        ordinals[index] = remainder as usize;
    }
}

pub fn combinations(cardinalities: &[usize]) -> u64 {
    cardinalities.iter().product::<usize>() as u64
}

pub struct Combinator<'a> {
    cardinalities: &'a [usize],
    combinations: u64,
}
impl<'a> Combinator<'a> {
    pub fn new(cardinalities: &'a [usize]) -> Self {
        let combinations = combinations(cardinalities);
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
            vectorize(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectorize_all() {
        let cardinalities = &[2, 3, 4];
        let mut outputs = vec![];
        let combinations = combinations(cardinalities);
        assert_eq!(24, combinations);
        for combination in 0..combinations {
            let mut ordinals = [0; 3];
            vectorize(cardinalities, combination, &mut ordinals);
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
}
