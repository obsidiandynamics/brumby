use std::collections::hash_map::Entry;
use std::error::Error;
use std::time::Instant;

use anyhow::anyhow;
use rustc_hash::FxHashMap;
use thiserror::Error;
use tracing::debug;
use brumby::capture::Capture;

use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, Overround, PriceBounds};
use brumby::probs::SliceExt;

use crate::domain::error::{InvalidOffer, InvalidOutcome, UnvalidatedOffer};
use crate::domain::{Offer, OfferCategory, OfferType, OutcomeType, Over, Period, Player};
use crate::interval;
use crate::interval::query::{isolate, requirements};
use crate::interval::{
    explore, BivariateProbs, Expansions, Exploration, PlayerProbs, PruneThresholds, TeamProbs,
    UnivariateProbs,
};

pub mod player_goal_fitter;
pub mod score_fitter;

#[derive(Debug, Error)]
pub enum FitError {
    #[error("{0}")]
    MissingOffer(#[from] MissingOffer),

    #[error("{0}")]
    UnmetRequirement(#[from] UnmetRequirement),

    #[error("{0}")]
    InvalidOffer(#[from] InvalidOffer),
}

#[derive(Debug, Error)]
pub enum MissingOffer {
    #[error("missing type {0:?}")]
    Type(OfferType),

    #[error("missing category {0:?}")]
    Category(OfferCategory),
}

fn get_offer<'a>(offers: &'a FxHashMap<OfferType, Offer>, offer_type: &OfferType) -> Result<UnvalidatedOffer<'a>, MissingOffer> {
    offers.get(offer_type).ok_or_else(|| MissingOffer::Type(offer_type.clone())).map(|offer| UnvalidatedOffer::from(Capture::Borrowed(offer)))
}

fn most_balanced_goals<'a>(
    offers: impl Iterator<Item = &'a Offer>,
    period: &Period,
) -> Option<(UnvalidatedOffer<'a>, &'a Over)> {
    let mut most_balanced = None;
    let mut most_balanced_diff = f64::MAX;
    for offer in offers {
        match &offer.offer_type {
            OfferType::TotalGoals(p, over) => {
                if p == period {
                    let diff = f64::abs(offer.market.prices[0] - offer.market.prices[1]);
                    if diff < most_balanced_diff {
                        most_balanced_diff = diff;
                        most_balanced = Some((offer, over));
                    }
                }
            }
            _ => {}
        }
    }
    most_balanced.map(|(offer, over)| (UnvalidatedOffer::from(Capture::Borrowed(offer)), over))
}

#[derive(Debug, Error)]
pub enum DerivationError {
    #[error("{0}")]
    UnmetRequirement(#[from] UnmetRequirement),

    #[error("{0}")]
    InvalidOutcome(#[from] InvalidOutcome),
}

#[derive(Debug, Error)]
pub enum UnmetRequirement {
    #[error("missing team goal probabilities")]
    TeamGoalProbabilities,

    #[error("missing team assist probabilities")]
    TeamAssistProbabilities,

    #[error("missing goal probability for {0:?}")]
    PlayerGoalProbability(Player),

    #[error("missing assist probability for {0:?}")]
    PlayerAssistProbability(Player),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ValidationError(#[from] pub Box<dyn Error>);
// impl ValidationError {
//     pub fn err<'a, S, T>(str: S) -> Result<T, ValidationError> where S: Into<&'a str> {
//         let str: &str = str.into();
//         Err(ValidationError::from(Box::from(str)))
//     }
// }

impl From<anyhow::Error> for ValidationError {
    fn from(value: anyhow::Error) -> Self {
        ValidationError(value.into())
    }
}

#[derive(Debug, Clone, Default)]
pub struct GoalProbs {
    pub h1: BivariateProbs,
    pub h2: BivariateProbs,
}

#[derive(Debug)]
pub struct Stub {
    pub offer_type: OfferType,
    pub outcomes: HashLookup<OutcomeType>,
    pub normal: f64,
    pub overround: Overround,
}
//
// #[derive(Debug)]
// pub enum OutcomeSet {
//     All,
//     Specific( HashLookup<OutcomeType>)
// }

#[derive(Debug)]
pub struct Config {
    pub intervals: u8,
    pub max_total_goals: u16,
}
impl Config {
    pub fn validate(&self) -> Result<(), ValidationError> {
        const MIN_INTERVALS: u8 = 4;
        if self.intervals < MIN_INTERVALS {
            return Err(anyhow!("number of intervals cannot be less than {MIN_INTERVALS}").into());
            //  ValidationError::err(&*format!("number of intervals cannot be less than {MIN_INTERVALS}"))?;
        }

        const MIN_MAX_TOTAL_GOALS: u16 = 6;
        if self.max_total_goals < MIN_MAX_TOTAL_GOALS {
            return Err(
                anyhow!("max total goals cannot be less than {MIN_MAX_TOTAL_GOALS}").into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Model {
    pub config: Config,
    pub goal_probs: Option<GoalProbs>,
    pub assist_probs: Option<UnivariateProbs>,
    pub player_probs: FxHashMap<Player, PlayerProbs>,
    pub offers: FxHashMap<OfferType, Offer>,
}
impl Model {
    pub fn derive(
        &mut self,
        stubs: &[Stub],
        price_bounds: &PriceBounds,
    ) -> Result<(), DerivationError> {
        let start = Instant::now();
        for stub in stubs {
            debug!("deriving {:?}", stub.offer_type);
            stub.offer_type.validate(&stub.outcomes)?;
            let offer = self.derive_offer(&stub, price_bounds)?;
            self.offers.insert(offer.offer_type.clone(), offer);
        }
        let elapsed = start.elapsed();
        debug!("derivation took {elapsed:?}");
        Ok(())
    }

    pub fn offers(&self) -> &FxHashMap<OfferType, Offer> {
        &self.offers
    }

    fn derive_offer(
        &mut self,
        stub: &Stub,
        price_bounds: &PriceBounds,
    ) -> Result<Offer, DerivationError> {
        let reqs = requirements(&stub.offer_type);
        self.ensure_team_requirements(&reqs)?;
        let requires_player_goal_probs = reqs.requires_player_goal_probs();
        let requires_player_assist_probs = reqs.requires_player_assist_probs();

        let team_probs = TeamProbs {
            h1_goals: self.goal_probs.clone().unwrap_or_default().h1,
            h2_goals: self.goal_probs.clone().unwrap_or_default().h2,
            assists: self.assist_probs.clone().unwrap_or_default(),
        };
        let prune_thresholds = PruneThresholds {
            max_total_goals: self.config.max_total_goals,
            min_prob: 0.0,
        };

        if requires_player_goal_probs || requires_player_assist_probs {
            // requires player probabilities — must be explored individually for each outcome
            todo!()
        } else {
            // doesn't require player probabilities — can be explored as a whole
            let exploration = explore(
                &interval::Config {
                    intervals: self.config.intervals,
                    team_probs,
                    player_probs: vec![],
                    prune_thresholds,
                    expansions: reqs,
                },
                0..self.config.intervals,
            );
            Ok(frame_prices_from_exploration(
                &exploration,
                &stub.offer_type,
                &stub.outcomes.items(),
                stub.normal,
                &stub.overround,
                price_bounds,
            ))
        }
    }

    fn ensure_team_requirements(&self, reqs: &Expansions) -> Result<(), UnmetRequirement> {
        if reqs.requires_team_goal_probs() {
            self.require_team_goal_probs()?;
        }
        if reqs.requires_team_assist_probs() {
            self.require_team_assist_probs()?
        }
        Ok(())
    }

    fn require_team_goal_probs(&self) -> Result<(), UnmetRequirement> {
        if self.goal_probs.is_none() {
            Err(UnmetRequirement::TeamGoalProbabilities)
        } else {
            Ok(())
        }
    }

    fn require_team_assist_probs(&self) -> Result<(), UnmetRequirement> {
        if self.assist_probs.is_none() {
            Err(UnmetRequirement::TeamAssistProbabilities)
        } else {
            Ok(())
        }
    }

    fn get_or_create_player(&mut self, player: Player) -> &mut PlayerProbs {
        match self.player_probs.entry(player) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(PlayerProbs::default()),
        }
        // if !self.player_probs.contains_key(player) {
        //     self.player_probs.insert(player.clone(), PlayerProbs::default());
        // }
        // self.player_probs.get_mut(player).unwrap()
    }
}

impl TryFrom<Config> for Model {
    type Error = ValidationError;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self {
            config,
            goal_probs: None,
            assist_probs: None,
            player_probs: Default::default(),
            offers: Default::default(),
        })
    }
}

fn frame_prices_from_exploration(
    exploration: &Exploration,
    offer_type: &OfferType,
    outcomes: &[OutcomeType],
    normal: f64,
    overround: &Overround,
    price_bounds: &PriceBounds,
) -> Offer {
    let mut probs = outcomes
        .iter()
        .map(|outcome_type| {
            isolate(
                offer_type,
                outcome_type,
                &exploration.prospects,
                &exploration.player_lookup,
            )
        })
        .collect::<Vec<_>>();
    probs.normalise(normal);
    let market = Market::frame(overround, probs, price_bounds);
    Offer {
        offer_type: offer_type.clone(),
        outcomes: HashLookup::from(outcomes.to_vec()),
        market,
    }
}
