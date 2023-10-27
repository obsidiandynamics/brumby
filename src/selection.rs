//! A [Selection] is a predicate applied to a podium slice. It is used to determine whether a given
//! runner has finished in a specific rank or among the top-_N_ placings.

use crate::capture::Capture;
use anyhow::{bail, Context};
use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
pub enum Selection {
    Span { runner: Runner, ranks: RangeInclusive<Rank> },
    Exact { runner: usize, rank: usize },
}
impl Selection {
    #[inline(always)]
    pub fn matches(&self, podium: &[usize]) -> bool {
        match self {
            Selection::Span {
                runner: Runner(runner),
                ranks,
            } => {
                let (start, end) = (ranks.start().as_index(), ranks.end().as_index());
                podium[start..=end]
                    .iter()
                    .any(|ranked_runner| ranked_runner == runner)
            },
            Selection::Exact { runner, rank } => podium[*rank] == *runner,
        }
    }
}

impl Display for Selection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Selection::Span { runner, ranks } => {
                write!(f, "{runner} in {}~{}", ranks.start(), ranks.end())
            }
            Selection::Exact { runner, rank } => {
                write!(f, "{runner} in {rank}")
            }
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

    pub const fn index(index: usize) -> Self {
        Self(index)
    }

    #[inline(always)]
    pub fn as_index(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn as_number(&self) -> usize {
        self.0 + 1
    }

    pub fn top(&self, highest_rank: Rank) -> Selection {
        Selection::Span {
            runner: self.clone(),
            ranks: Rank::first()..=highest_rank,
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

#[derive(Debug, PartialEq, Clone)]
pub struct Rank(usize);
impl Rank {
    pub fn number(number: usize) -> Self {
        Self::try_number(number).unwrap()
    }

    pub const fn first() -> Self {
        Self::index(0)
    }

    pub fn try_number(number: usize) -> anyhow::Result<Self> {
        if number == 0 {
            bail!("invalid rank number");
        }
        Ok(Self(number - 1))
    }

    pub const fn index(index: usize) -> Self {
        Self(index)
    }

    #[inline(always)]
    pub fn as_index(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn as_number(&self) -> usize {
        self.0 + 1
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.as_number())
    }
}

pub type Selections<'a> = Capture<'a, Vec<Selection>, [Selection]>;

impl<'a> FromStr for Selections<'a> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let frags = s.split('/');
        let mut selections = vec![];
        for (rank, frag) in frags.enumerate() {
            if !frag.is_empty() {
                let coranked = frag.split('+');
                for runner in coranked {
                    let runner = Runner::from_str(runner)?;
                    selections.push(Selection::Span {
                        runner,
                        ranks: Rank::first()..=Rank::index(rank),
                    })
                }
            }
        }
        Ok(selections.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn top() {
        assert!(Selection::Span {
            runner: Runner::index(5),
            ranks: Rank::first()..=Rank::number(1)
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: Runner::index(6),
            ranks: Rank::first()..=Rank::number(1)
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Span {
            runner: Runner::index(6),
            ranks: Rank::first()..=Rank::number(2)
        }
        .matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Span {
            runner: Runner::index(7),
            ranks: Rank::first()..=Rank::number(2)
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
    fn rank_as_index() {
        assert_eq!(6, Rank::number(7).as_index());
        assert_eq!(6, Rank::index(6).as_index());
    }

    #[test]
    fn rank_display() {
        let display = format!("{}", Rank::number(7));
        assert_eq!("@7", display);
    }

    #[test]
    #[should_panic = "invalid rank number"]
    fn rank_invalid_number() {
        Rank::number(0);
    }

    #[test]
    fn selections_from_str() {
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(Rank::number(1)),
                Runner::number(8).top(Rank::number(2)),
                Runner::number(9).top(Rank::number(3))
            ]),
            Selections::from_str("r7/r8/r9").unwrap()
        );
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(Rank::number(1)),
                Runner::number(8).top(Rank::number(3)),
                Runner::number(9).top(Rank::number(3))
            ]),
            Selections::from_str("r7//r8+r9").unwrap()
        );
        assert_eq!(
            Selections::Owned(vec![
                Runner::number(7).top(Rank::number(3)),
                Runner::number(8).top(Rank::number(3)),
                Runner::number(9).top(Rank::number(3))
            ]),
            Selections::from_str("//r7+r8+r9").unwrap()
        );
    }

    #[test]
    fn selections_clone() {
        let selections = Selections::Owned(vec![Runner::number(7).top(Rank::number(3))]);
        assert_eq!(
            Selections::Owned(vec![Runner::number(7).top(Rank::number(3))]),
            selections.clone()
        );
    }
}
