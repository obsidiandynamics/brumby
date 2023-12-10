pub struct Iter<'a> {
    assist_prob: f64,
    assisters: &'a[(usize, f64)],
    scorer_index: usize,
    other_player_index: usize,
    remaining_player_assist_prob: f64,
    pos: usize
}
impl<'a> Iter<'a> {
    pub fn new(assist_prob: f64,
               assisters: &'a[(usize, f64)],
               scorer_index: usize) -> Self {
        Self {
            assist_prob,
            assisters,
            scorer_index,
            other_player_index: assisters[assisters.len() - 1].0,
            remaining_player_assist_prob: 1.0,
            pos: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Option<usize>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let response = if self.pos == self.assisters.len() + 1 {
                // iterator exhausted
                None
            } else if self.pos == self.assisters.len() {
                // probability of no assist happening
                self.pos += 1;
                Some((None, 1.0 - self.assist_prob))
            } else {
                // probability of a player assisting
                let (assister, player_assist_prob) = {
                    let (assister, player_assist_prob) = self.assisters[self.pos];
                    // scorer cannot assist to self, unless assister is 'other' player
                    if assister != self.other_player_index && assister == self.scorer_index {
                        self.pos += 1;
                        self.assisters[self.pos]
                    } else {
                        (assister, player_assist_prob)
                    }
                };
                self.pos += 1;

                let player_assist_prob = if assister == self.other_player_index {
                    self.remaining_player_assist_prob
                } else {
                    self.remaining_player_assist_prob -= player_assist_prob;
                    player_assist_prob
                };
                Some((Some(assister), self.assist_prob * player_assist_prob))
            };

            match response {
                None => return None,
                Some((_, prob)) if prob > 0.0 => return response,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use super::*;

    #[test]
    fn scored_by_player_not_among_assisters() {
        let assisters = [
            (10, 0.1),
            (20, 0.2),
            (99, f64::NAN),
        ];
        let mut iter = Iter::new(0.7, &assisters, 5);
        assert_relative_eq(Some((Some(10), 0.07)), iter.next());
        assert_relative_eq(Some((Some(20), 0.14)), iter.next());
        assert_relative_eq(Some((Some(99), 0.49)), iter.next());
        assert_relative_eq(Some((None, 0.3)), iter.next());
        assert_relative_eq(None, iter.next());
    }

    #[test]
    fn scored_by_player_among_assisters() {
        let assisters = [
            (10, 0.1),
            (20, 0.2),
            (99, f64::NAN),
        ];
        let mut iter = Iter::new(0.7, &assisters, 20);
        assert_relative_eq(Some((Some(10), 0.07)), iter.next());
        assert_relative_eq(Some((Some(99), 0.63)), iter.next());
        assert_relative_eq(Some((None, 0.3)), iter.next());
        assert_relative_eq(None, iter.next());
    }

    #[test]
    fn scored_by_other() {
        let assisters = [
            (10, 0.1),
            (20, 0.2),
            (99, f64::NAN),
        ];
        let mut iter = Iter::new(0.7, &assisters, 99);
        assert_relative_eq(Some((Some(10), 0.07)), iter.next());
        assert_relative_eq(Some((Some(20), 0.14)), iter.next());
        assert_relative_eq(Some((Some(99), 0.49)), iter.next());
        assert_relative_eq(Some((None, 0.3)), iter.next());
        assert_relative_eq(None, iter.next());
    }

    #[test]
    fn skip_zero_prob() {
        let assisters = [
            (10, 0.1),
            (20, 0.2),
            (99, f64::NAN),
        ];
        let mut iter = Iter::new(1.0, &assisters, 5);
        assert_relative_eq(Some((Some(10), 0.1)), iter.next());
        assert_relative_eq(Some((Some(20), 0.2)), iter.next());
        assert_relative_eq(Some((Some(99), 0.7)), iter.next());
        assert_relative_eq(None, iter.next());
    }

    fn assert_relative_eq(left: Option<(Option<usize>, f64)>, right: Option<(Option<usize>, f64)>) {
        match left {
            None => {
                match right {
                    None => {}
                    Some(_) => {
                        panic!("left: {left:?}, right: {right:?}");
                    }
                }
            }
            Some(left_player_prob) => {
                match right {
                    None => {
                        panic!("left: {left:?}, right: {right:?}");
                    }
                    Some(right_player_prob) => {
                        assert_eq!(left_player_prob.0, right_player_prob.0, "left: {left:?}, right: {right:?}");
                        assert_float_relative_eq!(left_player_prob.1, right_player_prob.1);
                    }
                }
            }
        }
    }
}