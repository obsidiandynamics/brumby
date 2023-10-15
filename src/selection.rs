//! A [Selection] is a predicate applied to a podium slice. It is used to determine whether a given
//! runner has finished in a specific position or among the top-_N_ placings.

use std::ops::Range;

#[derive(Debug)]
pub enum Selection {
    Span { runner: usize, ranks: Range<usize> },
    Exact { runner: usize, rank: usize },
}
impl Selection {
    #[inline(always)]
    pub fn matches(&self, podium: &[usize]) -> bool {
        match self {
            Selection::Span { runner, ranks } => {
                for ranked_runner in podium[ranks.start..ranks.end].iter() {
                    if ranked_runner == runner {
                        return true;
                    }
                }
                false
            }
            Selection::Exact { runner, rank } => podium[*rank] == *runner,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::selection::Selection;

    #[test]
    fn top() {
        assert!(Selection::Span {
            runner: 5,
            ranks: 0..1
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: 6,
            ranks: 0..1
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Span {
            runner: 6,
            ranks: 0..2
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: 7,
            ranks: 0..2
        }
        .matches(&vec![5, 6, 7, 8]));
    }

    #[test]
    fn exact() {
        assert!(Selection::Exact { runner: 5, rank: 0 }.matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Exact { runner: 6, rank: 1 }.matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Exact { runner: 7, rank: 0 }.matches(&vec![5, 6, 7, 8]));
    }
}
