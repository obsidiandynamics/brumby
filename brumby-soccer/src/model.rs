use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use thiserror::Error;
use brumby::hash_lookup::HashLookup;

use crate::domain::{Offer, OfferCategory, OfferType, OutcomeType, Player};
use crate::domain::error::{InvalidOffer, InvalidOutcome};
use crate::interval::{BivariateProbs, PlayerProbs};

pub mod score_fitter;

#[derive(Debug, Error)]
pub enum FitError {
    #[error("{0}")]
    MissingOffer(#[from] MissingOffer),
    // #[error("other: {0}")]
    // Other(#[from] Box<dyn Error>)
}

#[derive(Debug, Error)]
pub enum MissingOffer {
    #[error("missing type {0:?}")]
    Type(OfferType),

    #[error("missing category {0:?}")]
    Category(OfferCategory),
}

#[derive(Debug, Error)]
pub enum DerivationError {
    #[error("{0}")]
    UnmetPrerequisite(#[from] UnmetPrerequisite),

    #[error("{0}")]
    InvalidOutcome(#[from] InvalidOutcome)
}

#[derive(Debug, Error)]
pub enum UnmetPrerequisite {
    #[error("missing team goal probabilities")]
    TeamGoalProbabilities,

    #[error("missing team assist probabilities")]
    TeamAssistProbabilities,

    #[error("missing goal probability for {0:?}")]
    PlayerGoalProbability(Player),

    #[error("missing assist probability for {0:?}")]
    PlayerAssistProbability(Player),
}

#[derive(Debug)]
pub struct GoalProbs {
    pub h1: BivariateProbs,
    pub h2: BivariateProbs,
}

#[derive(Debug)]
pub struct Derivation {
    offer_type: OfferType, 
    outcomes: OutcomeSet
}

#[derive(Debug)]
pub enum OutcomeSet {
    All,
    Specific( HashLookup<OutcomeType>)
}

#[derive(Debug)]
pub struct Model {
    pub goal_probs: Option<GoalProbs>,
    pub assist_probs: Option<BivariateProbs>,
    pub player_probs: FxHashMap<Player, PlayerProbs>,
    pub offers: FxHashMap<OfferType, Offer>,
}
impl Model {
    pub fn new() -> Self {
        Self {
            goal_probs: None,
            assist_probs: None,
            player_probs: Default::default(),
            offers: Default::default(),
        }
    }

    pub fn derive(&mut self, derivations: &[Derivation]) -> Result<(), DerivationError> {
        for derivation in derivations {
            let outcomes = match &derivation.outcomes {
                OutcomeSet::All => {
                    todo!()
                }
                OutcomeSet::Specific(outcomes) => {
                    derivation.offer_type.validate(outcomes)?;
                    outcomes
                }
            };
        }
        todo!()
    }

    fn get_or_create_player(&mut self, player: Player) -> &mut PlayerProbs {
        match self.player_probs.entry(player) {
            Entry::Occupied(entry) => {
                entry.into_mut()
            }
            Entry::Vacant(entry) => {
                entry.insert(PlayerProbs::default())
            }
        }
        // if !self.player_probs.contains_key(player) {
        //     self.player_probs.insert(player.clone(), PlayerProbs::default());
        // }
        // self.player_probs.get_mut(player).unwrap()
    }
}
