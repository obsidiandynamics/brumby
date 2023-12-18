use std::iter::{Filter, Zip};
use std::slice::Iter;

use brumby::hash_lookup::HashLookup;
use brumby::market::Market;

pub mod error;

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
impl OfferType {
    pub fn category(&self) -> OfferCategory {
        match self {
            OfferType::HeadToHead(_) => OfferCategory::HeadToHead,
            OfferType::TotalGoals(_, _) => OfferCategory::TotalGoals,
            OfferType::CorrectScore(_) => OfferCategory::CorrectScore,
            OfferType::DrawNoBet => OfferCategory::DrawNoBet,
            OfferType::AnytimeGoalscorer => OfferCategory::AnytimeGoalscorer,
            OfferType::FirstGoalscorer => OfferCategory::FirstGoalscorer,
            OfferType::PlayerShotsOnTarget(_) => OfferCategory::PlayerShotsOnTarget,
            OfferType::AnytimeAssist => OfferCategory::AnytimeAssist
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OfferCategory {
    HeadToHead,
    TotalGoals,
    CorrectScore,
    DrawNoBet,
    AnytimeGoalscorer,
    FirstGoalscorer,
    PlayerShotsOnTarget,
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
}