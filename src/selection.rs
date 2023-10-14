#[derive(Debug)]
pub enum Selection {
    Top { runner: usize, rank: usize },
    Exact { runner: usize, rank: usize },
}
impl Selection {
    pub fn matches(&self, podium: &[usize]) -> bool {
        match self {
            Selection::Top { runner, rank } => {
                for r in 0..=*rank {
                    if podium[r] == *runner {
                        return true;
                    }
                }
            }
            Selection::Exact { runner, rank} => {
                return podium[*rank] == *runner;
            }
        }
        false
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