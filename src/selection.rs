//! A [Selection] is a predicate applied to a podium slice. It is used to determine whether a given
//! runner has finished in a specific position or among the top-_N_ placings.

#[derive(Debug)]
pub enum Selection {
    Top { runner: usize, rank: usize },
    Exact { runner: usize, rank: usize },
}
impl Selection {
    #[inline(always)]
    pub fn matches(&self, podium: &[usize]) -> bool {
        match self {
            Selection::Top { runner, rank } => {
                for ranked_runner in podium[..=*rank].iter() {
                    if ranked_runner == runner {
                        return true;
                    }
                }
                false
            }
            Selection::Exact { runner, rank} => {
                podium[*rank] == *runner
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::selection::Selection;

    #[test]
    fn top() {
        assert!(Selection::Top { runner: 5, rank: 0 }.matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Top { runner: 6, rank: 0 }.matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Top { runner: 6, rank: 1 }.matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Top { runner: 7, rank: 1 }.matches(&vec![5, 6, 7, 8]));
    }

    #[test]
    fn exact() {
        assert!(Selection::Exact { runner: 5, rank: 0 }.matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Exact { runner: 6, rank: 1 }.matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Exact { runner: 7, rank: 0 }.matches(&vec![5, 6, 7, 8]));
    }
}