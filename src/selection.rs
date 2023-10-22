//! A [Selection] is a predicate applied to a podium slice. It is used to determine whether a given
//! runner has finished in a specific position or among the top-_N_ placings.

use crate::capture::Capture;
use anyhow::{bail, Context};
use std::fmt::{Display, Formatter};
use std::ops::Range;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Selection {
    Span { runner: Runner, ranks: Range<usize> },
    Exact { runner: usize, rank: usize },
}
impl Selection {
    #[inline(always)]
    pub fn matches(&self, podium: &[usize]) -> bool {
        match self {
            Selection::Span {
                runner: Runner(runner),
                ranks,
            } => podium[ranks.start..ranks.end]
                .iter()
                .any(|ranked_runner| ranked_runner == runner),
            Selection::Exact { runner, rank } => podium[*rank] == *runner,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Runner(usize);

impl Runner {
    pub fn number(number: usize) -> Self {
        Self::try_number(number).unwrap()
    }

    pub fn try_number(number: usize) -> anyhow::Result<Self> {
        if number == 0 {
            bail!("invalid runner number");
        }
        Ok(Self(number - 1))
    }

    pub fn index(index: usize) -> Self {
        Self(index)
    }

    pub fn as_index(&self) -> usize {
        self.0
    }

    pub fn as_number(&self) -> usize {
        self.0 + 1
    }

    pub fn top(&self, podium_exc: usize) -> Selection {
        Selection::Span {
            runner: self.clone(),
            ranks: 0..podium_exc,
        }
    }
}

impl Display for Runner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.as_number())
    }
}

impl FromStr for Runner {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let first_char = chars.next().context("no characters to parse")?;
        if first_char != 'r' {
            bail!("first character must be 'r'");
        }
        let remaining = chars.as_str();
        let runner_number: usize = remaining.parse()?;
        Runner::try_number(runner_number)
    }
}

pub type Selections<'a> = Capture<'a, Vec<Selection>, [Selection]>;

impl<'a> FromStr for Selections<'a> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let frags = s.split("/");
        let mut selections = vec![];
        for (rank, frag) in frags.enumerate() {
            if !frag.is_empty() {
                let coranked = frag.split("+");
                for runner in coranked {
                    let runner = Runner::from_str(runner)?;
                    selections.push(Selection::Span {
                        runner,
                        ranks: 0..rank + 1,
                    })
                }
            }
        }
        Ok(selections.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::selection::{Runner, Selection, Selections};
    use std::str::FromStr;

    #[test]
    fn top() {
        assert!(Selection::Span {
            runner: Runner::index(5),
            ranks: 0..1
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: Runner::index(6),
            ranks: 0..1
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Span {
            runner: Runner::index(6),
            ranks: 0..2
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: Runner::index(7),
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

    #[test]
    fn runner_as_index() {
        assert_eq!(6, Runner::number(7).as_index());
        assert_eq!(6, Runner::index(6).as_index());
    }

    #[test]
    fn runner_display() {
        let display = format!("{}", Runner::number(7));
        assert_eq!("r7", display);
    }

    #[test]
    #[should_panic = "invalid runner number"]
    fn runner_invalid_number() {
        Runner::number(0);
    }

    #[test]
    fn runner_from_str() {
        assert_eq!(Runner::index(6), Runner::from_str("r7").unwrap());

        assert_eq!(
            "no characters to parse",
            Runner::from_str("").err().unwrap().to_string()
        );

        assert_eq!(
            "first character must be 'r'",
            Runner::from_str("g").err().unwrap().to_string()
        );

        assert_eq!(
            "invalid digit found in string",
            Runner::from_str("rX").err().unwrap().to_string()
        );
    }

    #[test]
    fn selections_from_str() {
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(1),
                Runner::number(8).top(2),
                Runner::number(9).top(3)
            ]),
            Selections::from_str("r7/r8/r9").unwrap()
        );
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(1),
                Runner::number(8).top(3),
                Runner::number(9).top(3)
            ]),
            Selections::from_str("r7//r8+r9").unwrap()
        );
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(3),
                Runner::number(8).top(3),
                Runner::number(9).top(3)
            ]),
            Selections::from_str("//r7+r8+r9").unwrap()
        );
    }
}
