use std::iter::{Filter, Zip};
use std::slice::Iter;

use thiserror::Error;

use brumby::hash_lookup::HashLookup;
use brumby::market::Market;

use crate::domain::assert::{ExtraneousOutcome, MisalignedOffer, MissingOutcome, WrongBooksum};

pub mod assert;
mod total_goals;

#[derive(Debug, Error)]
pub enum InvalidOffer {
    #[error("misaligned offer: {0}")]
    MisalignedOffer(#[from] MisalignedOffer),

    #[error("{0}")]
    MissingOutcome(#[from] MissingOutcome),

    #[error("{0}")]
    ExtraneousOutcome(#[from] ExtraneousOutcome),

    #[error("wrong booksum: {0}")]
    WrongBooksum(#[from] WrongBooksum)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub home: u8,
    pub away: u8,
}
impl Score {
    pub fn new(home: u8, away: u8) -> Self {
        Self { home, away }
    }

    pub fn nil_all() -> Self {
        Self {
            home: 0,
            away: 0
        }
    }

    pub fn total(&self) -> u16 {
        (self.home + self.away) as u16
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Over(pub u8);

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Under(pub u8);

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Period {
    FirstHalf,
    SecondHalf,
    FullTime
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OfferType {
    HeadToHead(Period),
    TotalGoals(Period, Over),
    CorrectScore(Period),
    DrawNoBet,
    AnytimeGoalscorer,
    FirstGoalscorer,
    PlayerShotsOnTarget(Over),
    AnytimeAssist
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Home,
    Away,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Player {
    Named(Side, String),
    Other
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutcomeType {
    Win(Side),
    Draw,
    Under(u8),
    Over(u8),
    Score(Score),
    Player(Player),
    None,
}

#[derive(Debug)]
pub struct Offer {
    pub offer_type: OfferType,
    pub outcomes: HashLookup<OutcomeType>,
    pub market: Market,
}
impl Offer {
    pub fn filter_outcomes_with_probs<F>(&self, filter: F) -> Filter<Zip<Iter<OutcomeType>, Iter<f64>>, F> where F: FnMut(&(&OutcomeType, &f64)) -> bool{
        self.outcomes.items().iter().zip(self.market.probs.iter()).filter(filter)
    }

    pub fn validate(&self) -> Result<(), InvalidOffer> {
        assert::OfferAlignmentAssertion::check(&self.outcomes.items(), &self.market.probs, &self.offer_type)?;
        match self.offer_type {
            OfferType::TotalGoals(_, _) => {
                total_goals::validate(self)
            }
            _ => Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;
    use brumby::market::Overround;
    use super::*;

    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn misaligned_offer() {
        let offer = Offer {
            offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
            outcomes: HashLookup::from(vec![OutcomeType::Over(2), OutcomeType::Under(3)]),
            market: Market::frame(&Overround::fair(), vec![0.4], &PRICE_BOUNDS),
        };
        assert_eq!("misaligned offer: 2:1 outcomes:probabilities mapped for TotalGoals(FullTime, Over(2))", offer.validate().unwrap_err().to_string());
    }
}