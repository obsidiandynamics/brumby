use std::iter::{Filter, Zip};
use std::slice::Iter;

use bincode::Encode;

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

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Encode)]
pub enum Side {
    Home,
    Away,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Encode)]
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
impl OutcomeType {
    pub fn get_player(&self) -> Option<&Player> {
        match self {
            OutcomeType::Player(player) => Some(player),
            _ => None
        }
    }
}

// #[derive(Debug, Error)]
// pub enum OfferSubsetError {
//     #[error("no outcomes remaining")]
//     NoOutcomesRemaining
// }

#[derive(Debug)]
pub struct Offer {
    pub offer_type: OfferType,
    pub outcomes: HashLookup<OutcomeType>,
    pub market: Market,
}
impl Offer {
    pub fn filter_outcomes_with_probs<F>(&self, filter: F) -> Filter<Zip<Iter<OutcomeType>, Iter<f64>>, F> where F: FnMut(&(&OutcomeType, &f64)) -> bool {
        self.outcomes.items().iter().zip(self.market.probs.iter()).filter(filter)
    }
    
    pub fn get_probability(&self, outcome: &OutcomeType) -> Option<f64> {
        self.outcomes.index_of(outcome).map(|index| self.market.probs[index])
    }

    pub fn subset(&self, mut filter: impl FnMut(&(&OutcomeType, &f64)) -> bool) -> Option<Offer> {
        let mut outcomes = Vec::with_capacity(self.outcomes.len());
        let mut probs = Vec::with_capacity(self.outcomes.len());
        let mut prices = Vec::with_capacity(self.outcomes.len());

        for (index, outcome) in self.outcomes.items().iter().enumerate() {
            let prob = self.market.probs[index];
            if filter(&(outcome, &prob)) {
                outcomes.push(outcome.clone());
                probs.push(prob);
                prices.push(self.market.prices[index]);
            }
        }

        if outcomes.is_empty() {
            None
        }  else {
            Some(Self {
                offer_type: self.offer_type.clone(),
                outcomes: HashLookup::from(outcomes),
                market: Market {
                    probs,
                    prices,
                    overround: self.market.overround.clone(),
                },
            })
        }
    }
}