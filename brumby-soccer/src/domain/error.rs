use crate::domain::{Offer, OfferType, OutcomeType, Player};
use brumby::hash_lookup::HashLookup;
use brumby::probs::SliceExt;
use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use rustc_hash::FxHashMap;
use thiserror::Error;
use crate::interval::PlayerProbs;

mod head_to_head;
mod total_goals;

#[derive(Debug, Error)]
pub enum InvalidOffer {
    #[error("{0}")]
    MisalignedOffer(#[from] MisalignedOffer),

    #[error("{0}")]
    InvalidOutcome(#[from] InvalidOutcome),

    #[error("{0}")]
    WrongBooksum(#[from] WrongBooksum),
}

impl Offer {
    pub fn validate(&self) -> Result<(), InvalidOffer> {
        OfferAlignmentAssertion::check(
            &self.outcomes.items(),
            &self.market.probs,
            &self.offer_type,
        )?;
        self.offer_type.validate(&self.outcomes)?;
        match self.offer_type {
            OfferType::TotalGoals(_, _) => total_goals::validate_probs(&self.offer_type, &self.market.probs),
            OfferType::HeadToHead(_) => head_to_head::validate_probs(&self.offer_type, &self.market.probs),
            _ => Ok(()),
        }
    }
}


#[derive(Debug, Error)]
pub enum InvalidOutcome {
    #[error("{0}")]
    MissingOutcome(#[from] MissingOutcome),

    #[error("{0}")]
    ExtraneousOutcome(#[from] ExtraneousOutcome),
}

impl OfferType {
    pub fn validate(&self, outcomes: &HashLookup<OutcomeType>) -> Result<(), InvalidOutcome> {
        match self {
            OfferType::TotalGoals(_, _) => total_goals::validate_outcomes(self, outcomes),
            OfferType::HeadToHead(_) => head_to_head::validate_outcomes(self, outcomes),
            _ => Ok(()),
        }
    }

    pub fn create_outcomes(&self, player_probs: &FxHashMap<Player, PlayerProbs>) {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum FitError {
    #[error("missing offer {0:?}")]
    MissingOffer(OfferType),
}

#[derive(Debug, Error)]
#[error("expected booksum in {assertion}, got {actual} for {offer_type:?}")]
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
    pub fn check(
        &self,
        outcomes: &HashLookup<OutcomeType>,
        offer_type: &OfferType,
    ) -> Result<(), MissingOutcome> {
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
    pub matcher: F,
}
impl<F: FnMut(&OutcomeType) -> bool> OutcomesMatchAssertion<F> {
    pub fn check(
        &mut self,
        outcomes: &[OutcomeType],
        offer_type: &OfferType,
    ) -> Result<(), ExtraneousOutcome> {
        let mismatched = outcomes.iter().find(|&outcome| !(self.matcher)(outcome));
        match mismatched {
            None => Ok(()),
            Some(mismatched_outcome) => Err(ExtraneousOutcome {
                outcome_type: mismatched_outcome.clone(),
                offer_type: offer_type.clone(),
            }),
        }
    }
}

// #[derive(Debug, Error)]
// pub enum IncompleteOutcomes {
//     #[error("{0}")]
//     MissingOutcome(#[from] MissingOutcome),
//
//     #[error("{0}")]
//     ExtraneousOutcome(#[from] ExtraneousOutcome),
// }
//
// impl From<IncompleteOutcomes> for InvalidOutcome {
//     fn from(value: IncompleteOutcomes) -> Self {
//         match value {
//             IncompleteOutcomes::MissingOutcome(nested) => nested.into(),
//             IncompleteOutcomes::ExtraneousOutcome(nested) => nested.into(),
//         }
//     }
// }

#[derive(Debug)]
pub struct OutcomesCompleteAssertion<'a> {
    pub outcomes: &'a [OutcomeType],
}
impl<'a> OutcomesCompleteAssertion<'a> {
    pub fn check(
        &self,
        outcomes: &HashLookup<OutcomeType>,
        offer_type: &OfferType,
    ) -> Result<(), InvalidOutcome> {
        OutcomesIntactAssertion {
            outcomes: self.outcomes,
        }
        .check(outcomes, offer_type)?;

        if outcomes.len() != self.outcomes.len() {
            Err(OutcomesMatchAssertion {
                matcher: |outcome| self.outcomes.contains(outcome),
            }
            .check(outcomes.items(), offer_type)
            .unwrap_err()
            .into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Over, Period, Side};
    use brumby::market::{Market, Overround};

    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn misaligned_offer() {
        let offer = Offer {
            offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
            outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
            market: Market::frame(&Overround::fair(), vec![0.4], &PRICE_BOUNDS),
        };
        assert_eq!(
            "2:1 outcomes:probabilities mapped for TotalGoals(FullTime, Over(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

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
                "expected booksum in 1.0..=2.0 ± 0.01, got 0.989 for FirstGoalscorer",
                err.to_string()
            );
        }
        {
            let err = assertion
                .check(&[2.0 + 0.011], &OfferType::FirstGoalscorer)
                .unwrap_err();
            assert_eq!(
                "expected booksum in 1.0..=2.0 ± 0.01, got 2.011 for FirstGoalscorer",
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
        assertion
            .check(
                &HashLookup::from(vec![
                    OutcomeType::Win(Side::Home),
                    OutcomeType::Win(Side::Away),
                ]),
                &OfferType::DrawNoBet,
            )
            .unwrap();
    }

    #[test]
    fn outcome_missing() {
        let assertion = OutcomesIntactAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        let err = assertion
            .check(
                &HashLookup::from([OutcomeType::Win(Side::Home)]),
                &OfferType::DrawNoBet,
            )
            .unwrap_err();
        assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
    }

    #[test]
    fn outcomes_match() {
        let mut assertion = OutcomesMatchAssertion {
            matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
        };
        assertion
            .check(
                &[
                    OutcomeType::Win(Side::Home),
                    OutcomeType::Win(Side::Away),
                    OutcomeType::Draw,
                ],
                &OfferType::HeadToHead(Period::FullTime),
            )
            .unwrap();
    }

    #[test]
    fn outcome_does_not_match() {
        let mut assertion = OutcomesMatchAssertion {
            matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
        };
        let err = assertion
            .check(
                &[OutcomeType::Win(Side::Home), OutcomeType::None],
                &OfferType::HeadToHead(Period::FullTime),
            )
            .unwrap_err();
        assert_eq!(
            "None does not belong in HeadToHead(FullTime)",
            err.to_string()
        );
    }

    #[test]
    fn outcomes_complete() {
        let assertion = OutcomesCompleteAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        assertion
            .check(
                &HashLookup::from([
                    OutcomeType::Win(Side::Home),
                    OutcomeType::Win(Side::Away),
                ]),
                &OfferType::DrawNoBet,
            )
            .unwrap();
    }

    #[test]
    fn outcomes_incomplete() {
        let assertion = OutcomesCompleteAssertion {
            outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        };
        {
            let err = assertion
                .check(
                    &HashLookup::from([OutcomeType::Win(Side::Home)]),
                    &OfferType::DrawNoBet,
                )
                .unwrap_err();
            assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
        }
        {
            let err = assertion
                .check(
                    &HashLookup::from([
                        OutcomeType::Win(Side::Home),
                        OutcomeType::Win(Side::Away),
                        OutcomeType::Draw,
                    ]),
                    &OfferType::DrawNoBet,
                )
                .unwrap_err();
            assert_eq!("Draw does not belong in DrawNoBet", err.to_string());
        }
    }
}
