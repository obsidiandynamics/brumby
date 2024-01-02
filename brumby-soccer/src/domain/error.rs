use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;

use thiserror::Error;

use brumby::capture::Capture;
use brumby::hash_lookup::HashLookup;
use brumby::probs::SliceExt;

use crate::domain::{Offer, OfferType, Outcome};

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

    #[error("{0}")]
    InvalidMarket(#[from] anyhow::Error),
}

impl Offer {
    pub fn validate(&self) -> Result<(), InvalidOffer> {
        self.market.validate()?;
        OfferAlignmentAssertion::check(
            self.outcomes.items(),
            &self.market.probs,
            &self.offer_type,
        )?;
        self.offer_type.validate_outcomes(&self.outcomes)?;
        match self.offer_type {
            OfferType::TotalGoals(_, _) => total_goals::validate_probs(&self.offer_type, &self.market.probs),
            OfferType::HeadToHead(_) => head_to_head::validate_probs(&self.offer_type, &self.market.probs),
            _ => Ok(()),
        }
    }
}

pub type OfferCapture<'a> = Capture<'a, Offer>;

/// Prevents accidental handling of an [Offer] before it has been validated.
#[derive(Debug)]
pub struct UnvalidatedOffer<'a>(OfferCapture<'a>);
impl<'a> UnvalidatedOffer<'a> {
    pub fn unchecked(self) -> OfferCapture<'a> {
        self.0
    }
}

impl<'a> From<OfferCapture<'a>> for UnvalidatedOffer<'a> {
    fn from(offer: OfferCapture<'a>) -> Self {
        Self(offer)
    }
}

impl<'a> TryFrom<UnvalidatedOffer<'a>> for OfferCapture<'a> {
    type Error = InvalidOffer;

    fn try_from(offer: UnvalidatedOffer<'a>) -> Result<Self, Self::Error> {
        offer.0.validate()?;
        Ok(offer.0)
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
    pub fn validate_outcomes(&self, outcomes: &HashLookup<Outcome>) -> Result<(), InvalidOutcome> {
        match self {
            OfferType::TotalGoals(_, _) => total_goals::validate_outcomes(self, outcomes),
            OfferType::HeadToHead(_) => head_to_head::validate_outcomes(self, outcomes),
            _ => Ok(()),
        }
    }

    pub fn validate_outcome(&self, outcome: &Outcome) -> Result<(), InvalidOutcome> {
        match self {
            OfferType::TotalGoals(_, _) => total_goals::validate_outcome(self, outcome),
            OfferType::HeadToHead(_) => head_to_head::validate_outcome(self, outcome),
            _ => Ok(()),
        }
    }
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
    const DEFAULT_TOLERANCE: f64 = 1e-3;

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
        outcomes: &[Outcome],
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
#[error("{outcome:?} missing from {offer_type:?}")]
pub struct MissingOutcome {
    pub outcome: Outcome,
    pub offer_type: OfferType,
}

#[derive(Debug)]
pub struct OutcomesIntactAssertion<'a> {
    pub outcomes: &'a [Outcome],
}
impl<'a> OutcomesIntactAssertion<'a> {
    pub fn check(
        &self,
        outcomes: &HashLookup<Outcome>,
        offer_type: &OfferType,
    ) -> Result<(), MissingOutcome> {
        for outcome in self.outcomes.iter() {
            if outcomes.index_of(outcome).is_none() {
                return Err(MissingOutcome {
                    outcome: outcome.clone(),
                    offer_type: offer_type.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("{outcome:?} does not belong in {offer_type:?}")]
pub struct ExtraneousOutcome {
    pub outcome: Outcome,
    pub offer_type: OfferType,
}

pub struct OutcomesMatchAssertion<F: FnMut(&Outcome) -> bool> {
    pub matcher: F,
}
impl<F: FnMut(&Outcome) -> bool> OutcomesMatchAssertion<F> {
    pub fn check(
        &mut self,
        outcomes: &[Outcome],
        offer_type: &OfferType,
    ) -> Result<(), ExtraneousOutcome> {
        let mismatched = outcomes.iter().find(|&outcome| !(self.matcher)(outcome));
        match mismatched {
            None => Ok(()),
            Some(mismatched_outcome) => Err(ExtraneousOutcome {
                outcome: mismatched_outcome.clone(),
                offer_type: offer_type.clone(),
            }),
        }
    }
}

#[derive(Debug)]
pub struct OutcomesCompleteAssertion<'a> {
    pub outcomes: &'a [Outcome],
}
impl<'a> OutcomesCompleteAssertion<'a> {
    pub fn check(
        &self,
        outcomes: &HashLookup<Outcome>,
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
mod tests;
