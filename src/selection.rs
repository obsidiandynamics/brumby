//! A [Selection] is a predicate applied to a podium slice. It is used to determine whether a given
//! runner has finished in a specific rank or among the top-_N_ placings.

use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use std::str::FromStr;

use anyhow::{bail, Context};

use crate::capture::Capture;
use crate::linear::matrix::Matrix;

#[derive(Debug, PartialEq, Clone)]
pub enum Selection {
    Span { runner: Runner, ranks: RangeInclusive<Rank> },
    Exact { runner: Runner, rank: Rank },
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
            Selection::Exact { runner, rank } => podium[rank.as_index()] == runner.as_index(),
        }
    }

    pub fn validate(&self, allowed_ranks: RangeInclusive<usize>, probs: &[f64]) -> Result<(), anyhow::Error> {
        let validate_runner = |runner : &Runner| {
            let runners = probs.len();
            let runner_index = runner.as_index();
            if runner_index > runners - 1 {
                bail!("invalid runner {runner}");
            }
            if probs[runner_index] == 0. {
                bail!("{runner} has a zero finishing probability");
            }
            Ok(())
        };

        match self {
            Selection::Span { runner, ranks} => {
                validate_runner(runner)?;
                if ranks.start().as_index() < *allowed_ranks.start() || ranks.end().as_index() > *allowed_ranks.end() {
                    bail!("invalid finishing ranks {}", ranks.display());
                }
            }
            Selection::Exact { runner, rank } => {
                validate_runner(runner)?;
                if !allowed_ranks.contains(&rank.as_index()) {
                    bail!("invalid finishing rank {rank}");
                }
            }
        }
        Ok(())
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

pub trait RangeInclusiveExt<T: Display> {
    fn display(&self) -> RangeInclusiveDisplay<T>;
}
impl<T: Display> RangeInclusiveExt<T> for RangeInclusive<T> {
    fn display(&self) -> RangeInclusiveDisplay<T> {
        RangeInclusiveDisplay { range: self }
    }
}

pub struct RangeInclusiveDisplay<'a, T: Display> {
    range: &'a RangeInclusive<T>
}

impl<'a, T: Display> Display for RangeInclusiveDisplay<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.range.start(), self.range.end())
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

/// Builds a `podium_places` x `num_runners` matrix populated with top-_N_ selections.
pub fn top_n_matrix(podium_places: usize, num_runners: usize) -> Matrix<Selections<'static>> {
    let mut scenarios = Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            scenarios[(rank, runner)] = vec![Runner::index(runner).top(Rank::index(rank))].into();
        }
    }
    scenarios
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

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
        assert!(Selection::Exact { runner: Runner::index(5), rank: Rank::index(0) }.matches(&vec![5, 6, 7, 8]));
        assert!(Selection::Exact { runner: Runner::index(6), rank: Rank::index(1) }.matches(&vec![5, 6, 7, 8]));
        assert!(!Selection::Exact { runner: Runner::index(7), rank: Rank::index(0) }.matches(&vec![5, 6, 7, 8]));
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

    #[test]
    fn validate_selection_exact() {
        let sel = Selection::Exact { runner: Runner::index(3), rank: Rank::index(2)};
        assert!(sel.validate(0..=2, &[0.1, 0.2, 0.3, 0.4]).is_ok());
        assert_eq!("invalid finishing rank @3", sel.validate(0..=1, &[0.1, 0.2, 0.3, 0.4]).err().unwrap().to_string());
        assert_eq!("invalid finishing rank @3", sel.validate(3..=4, &[0.1, 0.2, 0.3, 0.4]).err().unwrap().to_string());
        assert_eq!("invalid runner r4", sel.validate(2..=2, &[0.1, 0.2, 0.3]).err().unwrap().to_string());
        assert_eq!("r4 has a zero finishing probability", sel.validate(2..=2, &[0.1, 0.2, 0.3, 0.0]).err().unwrap().to_string());
    }

    #[test]
    fn validate_selection_span() {
        let sel = Selection::Span { runner: Runner::index(3), ranks: Rank::index(2)..=Rank::index(3)};
        assert!(sel.validate(0..=3, &[0.1, 0.2, 0.3, 0.4]).is_ok());
        assert_eq!("invalid finishing ranks @3-@4", sel.validate(0..=1, &[0.1, 0.2, 0.3, 0.4]).err().unwrap().to_string());
        assert_eq!("invalid finishing ranks @3-@4", sel.validate(4..=5, &[0.1, 0.2, 0.3, 0.4]).err().unwrap().to_string());
        assert_eq!("invalid runner r4", sel.validate(2..=2, &[0.1, 0.2, 0.3]).err().unwrap().to_string());
        assert_eq!("r4 has a zero finishing probability", sel.validate(2..=2, &[0.1, 0.2, 0.3, 0.0]).err().unwrap().to_string());
    }
}
