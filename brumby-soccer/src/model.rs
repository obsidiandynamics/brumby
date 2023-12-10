use rustc_hash::FxHashMap;

use crate::domain::{Offer, OfferType, Player};
use crate::interval::{PlayerProbs, BivariateProbs};

pub mod period_fitter;

#[derive(Debug)]
pub struct SplitScoringProbs {
    pub h1: BivariateProbs,
    pub h2: BivariateProbs,
}

#[derive(Debug)]
pub struct Model {
    pub scoring_probs: Option<SplitScoringProbs>,
    pub player_probs: FxHashMap<Player, PlayerProbs>,
    pub offers: FxHashMap<OfferType, Offer>
}
impl Model {
    pub fn new() -> Self {
        Self {
            scoring_probs: None,
            player_probs: Default::default(),
            offers: Default::default(),
        }
    }
}