use crate::domain::{InvalidOffer, OfferType, OutcomeType};
use brumby::probs::SliceExt;
use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use thiserror::Error;
use brumby::hash_lookup::HashLookup;

#[derive(Debug, Error)]
#[error("expected {assertion}, got {actual} for {offer_type:?}")]
pub struct WrongBooksum {
    pub assertion: BooksumAssertion,
    pub actual: f64,
    pub offer_type: OfferType,
}

#[derive(Debug, Clone)]
pub struct BooksumAssertion {
    pub expected: RangeInclusive<f64>,
    pub tolerance: f64,
}
impl BooksumAssertion {
    const DEFAULT_TOLERANCE: f64 = 1e-6;

    pub fn with_default_tolerance(expected: RangeInclusive<f64>) -> Self {
        Self {
            expected,
            tolerance: Self::DEFAULT_TOLERANCE,
        }
    }

    pub fn check(&self, probs: &[f64], offer_type: &OfferType) -> Result<(), WrongBooksum> {
        let actual = probs.sum();
        if actual < *self.expected.start() - self.tolerance
            || actual > *self.expected.end() + self.tolerance
        {
            Err(WrongBooksum {
                assertion: self.clone(),
                actual,
                offer_type: offer_type.clone(),
            })
        } else {
            Ok(())
        }
    }
}

impl Display for BooksumAssertion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} ± {}", self.expected, self.tolerance)
    }
}

#[derive(Debug, Error)]
#[error("{outcomes}:{probs} outcomes:probabilities mapped for {offer_type:?}")]
pub struct MisalignedOffer {
    outcomes: usize,
    probs: usize,
    offer_type: OfferType,
}

pub struct OfferAlignmentAssertion;
impl OfferAlignmentAssertion {
    pub fn check(
        outcomes: &[OutcomeType],
        probs: &[f64],
        offer_type: &OfferType,
    ) -> Result<(), MisalignedOffer> {
        if outcomes.len() != probs.len() {
            Err(MisalignedOffer {
                outcomes: outcomes.len(),
                probs: probs.len(),
                offer_type: offer_type.clone(),
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Error)]
#[error("{outcome_type:?} missing from {offer_type:?}")]
pub struct MissingOutcome {
    outcome_type: OutcomeType,
    offer_type: OfferType,
}

#[derive(Debug)]
pub struct OutcomesIntactAssertion<'a> {
    pub outcomes: &'a [OutcomeType],
}
impl<'a> OutcomesIntactAssertion<'a> {
    pub fn check(&self, outcomes: &HashLookup<OutcomeType>, offer_type: &OfferType) -> Result<(), MissingOutcome> {
        for outcome in self.outcomes.iter() {
            if outcomes.index_of(outcome).is_none() {
                return Err(MissingOutcome {
                    outcome_type: outcome.clone(),
                    offer_type: offer_type.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("{outcome_type:?} does not belong in {offer_type:?}")]
pub struct ExtraneousOutcome {
    outcome_type: OutcomeType,
    offer_type: OfferType,
}

pub struct OutcomesMatchAssertion<F: FnMut(&OutcomeType) -> bool> {
    pub matcher: F
}
impl<F: FnMut(&OutcomeType) -> bool> OutcomesMatchAssertion<F> {
    pub fn check(&mut self, outcomes: &[OutcomeType], offer_type: &OfferType) -> Result<(), ExtraneousOutcome> {
        let mismatched = outcomes.iter().find(|&outcome| !(self.matcher)(outcome));
        match mismatched {
            None => Ok(()),
            Some(mismatched_outcome) => Err(ExtraneousOutcome {
                outcome_type: mismatched_outcome.clone(),
                offer_type: offer_type.clone(),
            })
        }
    }
}

#[derive(Debug, Error)]
pub enum IncompleteOutcomes {
    #[error("{0}")]
    MissingOutcome(#[from] MissingOutcome),

    #[error("{0}")]
    ExtraneousOutcome(#[from] ExtraneousOutcome),
}

impl From<IncompleteOutcomes> for InvalidOffer {
    fn from(value: IncompleteOutcomes) -> Self {
        match value {
            IncompleteOutcomes::MissingOutcome(nested) => nested.into(),
            IncompleteOutcomes::ExtraneousOutcome(nested) => nested.into()
        }
    }
}

#[derive(Debug)]
pub struct OutcomesCompleteAssertion<'a> {
    pub outcomes: &'a [OutcomeType],
}
impl<'a> OutcomesCompleteAssertion<'a> {
    pub fn check(&self, outcomes: &HashLookup<OutcomeType>, offer_type: &OfferType) -> Result<(), IncompleteOutcomes> {
        OutcomesIntactAssertion {
            outcomes: self.outcomes,
        }.check(outcomes, offer_type)?;

        if outcomes.len() != self.outcomes.len() {
            Err(OutcomesMatchAssertion {
                matcher: |outcome| self.outcomes.contains(outcome),
            }.check(outcomes.items(), offer_type).unwrap_err().into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Period, Side};

    #[test]
    fn booksum_within_expected() {
        let assertion = BooksumAssertion {
            expected: 1.0..=2.0,
            tolerance: 0.01,
        };
        assertion
            .check(&[1.0 - 0.01], &OfferType::FirstGoalscorer)
            .unwrap();
        assertion
            .check(&[2.0 + 0.01], &OfferType::FirstGoalscorer)
            .unwrap();
    }

    #[test]
    fn booksum_outside_expected() {
        let assertion = BooksumAssertion {
            expected: 1.0..=2.0,
            tolerance: 0.01,
        };
        {
            let err = assertion
                .check(&[1.0 - 0.011], &OfferType::FirstGoalscorer)
                .unwrap_err();
            assert_eq!(
                "expected 1.0..=2.0 ± 0.01, got 0.989 for FirstGoalscorer",
                err.to_string()
            );
        }
        {
            let err = assertion
                .check(&[2.0 + 0.011], &OfferType::FirstGoalscorer)
                .unwrap_err();
            assert_eq!(
                "expected 1.0..=2.0 ± 0.01, got 2.011 for FirstGoalscorer",
                err.to_string()
            );
        }
    }

    #[test]
    fn alignment_correct() {
        OfferAlignmentAssertion::check(
            &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
            &[0.5, 0.5],
            &OfferType::DrawNoBet,
        )
        .unwrap();
    }

    #[test]
    fn alignment_incorrect() {
        let err = OfferAlignmentAssertion::check(
            &[OutcomeType::Win(Side::Home)],
            &[0.5, 0.5],
            &OfferType::DrawNoBet,
        )
        .unwrap_err();
        assert_eq!(
            "1:2 outcomes:probabilities mapped for DrawNoBet",
            err.to_string()
        );
    }

    #[test]
    fn outcomes_intact() {
        let assertion = OutcomesIntactAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        assertion.check(&HashLookup::from(vec![OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)]), &OfferType::DrawNoBet).unwrap();
    }

    #[test]
    fn outcome_missing() {
        let assertion = OutcomesIntactAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        let err = assertion.check(&HashLookup::from(vec![OutcomeType::Win(Side::Home)]), &OfferType::DrawNoBet).unwrap_err();
        assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
    }

    #[test]
    fn outcomes_match() {
        let mut assertion = OutcomesMatchAssertion {
            matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
        };
        assertion.check(&[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away), OutcomeType::Draw], &OfferType::HeadToHead(Period::FullTime)).unwrap();
    }

    #[test]
    fn outcome_does_not_match() {
        let mut assertion = OutcomesMatchAssertion {
            matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
        };
        let err = assertion.check(&[OutcomeType::Win(Side::Home), OutcomeType::None], &OfferType::HeadToHead(Period::FullTime)).unwrap_err();
        assert_eq!("None does not belong in HeadToHead(FullTime)", err.to_string());
    }

    #[test]
    fn outcomes_complete() {
        let assertion = OutcomesCompleteAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        assertion.check(&HashLookup::from(vec![OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)]), &OfferType::DrawNoBet).unwrap();
    }

    #[test]
    fn outcomes_incomplete() {
        let assertion = OutcomesCompleteAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        {
            let err = assertion.check(&HashLookup::from(vec![OutcomeType::Win(Side::Home)]), &OfferType::DrawNoBet).unwrap_err();
            assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
        }
        {
            let err = assertion.check(&HashLookup::from(vec![OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away), OutcomeType::Draw]), &OfferType::DrawNoBet).unwrap_err();
            assert_eq!("Draw does not belong in DrawNoBet", err.to_string());
        }
    }
}
