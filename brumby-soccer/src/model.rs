use std::error::Error;
use rustc_hash::FxHashMap;
use thiserror::Error;

use crate::domain::{Offer, OfferType, Player};
use crate::interval::{PlayerProbs, BivariateProbs};

pub mod score_fitter;

#[derive(Debug, Error)]
pub enum FitError {
    #[error("missing offer {0:?}")]
    MissingOffer(OfferType),

    #[error("other: {0}")]
    Other(#[from] Box<dyn Error>)
}

#[derive(Debug)]
pub struct GoalProbs {
    pub h1: BivariateProbs,
    pub h2: BivariateProbs,
}

#[derive(Debug)]
pub struct Model {
    pub goal_probs: Option<GoalProbs>,
    pub assist_probs: Option<BivariateProbs>,
    pub player_probs: FxHashMap<Player, PlayerProbs>,
    pub offers: FxHashMap<OfferType, Offer>
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
}