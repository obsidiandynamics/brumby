use std::collections::hash_map::Entry;
use std::error::Error;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use rustc_hash::FxHashMap;
use thiserror::Error;
use tracing::{debug, trace};

use brumby::capture::Capture;
use brumby::derived_price::DerivedPrice;
use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, Overround, PriceBounds};
use brumby::probs::SliceExt;
use brumby::stack_vec::FromIteratorResult;
use brumby::sv;
use brumby::timed::Timed;

use crate::domain::error::{InvalidOffer, InvalidOutcome, MissingOutcome, UnvalidatedOffer};
use crate::domain::{Offer, OfferCategory, OfferType, Outcome, Over, Period, Player};
use crate::interval;
use crate::interval::query::{isolate, requirements};
use crate::interval::{
    query, BivariateProbs, Expansions, Exploration, PlayerProbs, PruneThresholds, TeamProbs,
    UnivariateProbs,
};
use crate::model::cache::{CacheStats, CacheableIntervalArgs, CachingContext};

mod cache;
pub mod player_assist_fitter;
pub mod player_goal_fitter;
pub mod score_fitter;

#[derive(Debug, Error)]
pub enum FitError {
    #[error("{0}")]
    MissingOffer(#[from] MissingOffer),

    #[error("{0}")]
    MissingOutcome(#[from] MissingOutcome),

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

fn get_offer<'a>(
    offers: &'a FxHashMap<OfferType, Offer>,
    offer_type: &OfferType,
) -> Result<UnvalidatedOffer<'a>, MissingOffer> {
    offers
        .get(offer_type)
        .ok_or_else(|| MissingOffer::Type(offer_type.clone()))
        .map(|offer| UnvalidatedOffer::from(Capture::Borrowed(offer)))
}

fn most_balanced_goals<'a>(
    offers: impl Iterator<Item = &'a Offer>,
    period: &Period,
) -> Option<(UnvalidatedOffer<'a>, &'a Over)> {
    let mut most_balanced = None;
    let mut most_balanced_diff = f64::MAX;
    for offer in offers {
        if let OfferType::TotalGoals(p, over) = &offer.offer_type {
            if p == period {
                let diff = f64::abs(offer.market.prices[0] - offer.market.prices[1]);
                if diff < most_balanced_diff {
                    most_balanced_diff = diff;
                    most_balanced = Some((offer, over));
                }
            }
        }
    }
    most_balanced.map(|(offer, over)| (UnvalidatedOffer::from(Capture::Borrowed(offer)), over))
}

fn get_or_create_player(
    player_probs: &mut FxHashMap<Player, PlayerProbs>,
    player: Player,
) -> &mut PlayerProbs {
    match player_probs.entry(player) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(PlayerProbs::default()),
    }
}

#[derive(Debug, Error)]
pub enum SingleDerivationError {
    #[error("{0}")]
    UnmetRequirement(#[from] UnmetRequirement),

    #[error("{0}")]
    InvalidOutcome(#[from] InvalidOutcome),
}

#[derive(Debug, Error)]
pub enum MultiDerivationError {
    #[error("{0}")]
    NoSelections(#[from] NoSelections),

    #[error("{0}")]
    UnmetRequirement(#[from] UnmetRequirement),

    #[error("{0}")]
    InvalidOutcome(#[from] InvalidOutcome),

    #[error("{0}")]
    TooManyPlayers(#[from] TooManyPlayers),

    #[error("{0}")]
    MissingDerivative(#[from] MissingDerivative),
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("no selections specified")]
pub struct NoSelections;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("at most {capacity} players supported")]
pub struct TooManyPlayers {
    pub capacity: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("missing {offer_type:?}/{outcome:?} among derivatives")]
pub struct MissingDerivative {
    pub offer_type: OfferType,
    pub outcome: Outcome,
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
    pub outcomes: HashLookup<Outcome>,
    pub normal: f64,
    pub overround: Overround,
}

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

// #[derive(Debug)]
// pub struct MultiPrice {
//     pub price: DerivedPrice,
//     pub relatedness: f64
// }

#[derive(Debug)]
pub struct MultiDerivation {
    pub quotation: DerivedPrice,
    pub redundancies: Vec<(OfferType, Outcome)>,
    pub relatedness: f64,
    pub fringes: FxHashMap<OfferType, Vec<Fringe>>,
}

#[derive(Debug)]
pub struct Fringe {
    pub outcome: Outcome,
    pub quotation: DerivedPrice,
    pub redundancies: Vec<(OfferType, Outcome)>,
    pub relatedness: f64,
}

#[derive(Debug, Clone)]
struct Selection {
    offer_type: OfferType,
    outcome: Outcome,
    single_overround: f64,
    single_prob: f64,
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
    pub fn offers(&self) -> &FxHashMap<OfferType, Offer> {
        &self.offers
    }

    pub fn derive(
        &mut self,
        stubs: &[Stub],
        price_bounds: &PriceBounds,
    ) -> Result<Timed<CacheStats>, SingleDerivationError> {
        Timed::result(|| {
            let mut caching_context = CachingContext::default();
            for stub in stubs {
                debug!(
                    "deriving {:?} ({} outcomes)",
                    stub.offer_type,
                    stub.outcomes.len()
                );
                stub.offer_type.validate_outcomes(&stub.outcomes)?;
                let start = Instant::now();
                let offer = self.derive_offer(stub, price_bounds, &mut caching_context)?;
                debug!(
                    "... took {:?}, progressive {:?}",
                    start.elapsed(),
                    caching_context.stats
                );
                self.offers.insert(offer.offer_type.clone(), offer);
            }
            Ok(caching_context.stats)
        })
    }

    fn derive_offer(
        &mut self,
        stub: &Stub,
        price_bounds: &PriceBounds,
        caching_context: &mut CachingContext,
    ) -> Result<Offer, SingleDerivationError> {
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

        let offer = if requires_player_goal_probs || requires_player_assist_probs {
            // requires player probabilities — must be explored individually for each outcome
            let mut probs = Vec::with_capacity(stub.outcomes.len());
            for outcome in &stub.outcomes {
                let player_probs = match outcome.get_player() {
                    None => sv![],
                    Some(player) => {
                        let mut player_probs = PlayerProbs::default();
                        if requires_player_goal_probs {
                            player_probs.goal = Some(self.require_player_goal_prob(player)?);
                        }
                        if requires_player_assist_probs {
                            player_probs.assist = Some(self.require_player_assist_prob(player)?);
                        }
                        sv![(player.clone(), player_probs)]
                    }
                };

                let exploration = caching_context.explore(CacheableIntervalArgs {
                    config: interval::Config {
                        intervals: self.config.intervals,
                        team_probs: team_probs.clone(),
                        player_probs,
                        prune_thresholds: prune_thresholds.clone(),
                        expansions: reqs.clone(),
                    },
                    include_intervals: 0..self.config.intervals,
                });

                let prob = isolate(
                    &stub.offer_type,
                    outcome,
                    &exploration.prospects,
                    &exploration.player_lookup,
                );
                probs.push(prob);
            }
            debug!("... normalizing: {} -> {}", probs.sum(), stub.normal);
            probs.normalise(stub.normal);
            let market = Market::frame(&stub.overround, probs, price_bounds);
            Offer {
                offer_type: stub.offer_type.clone(),
                outcomes: stub.outcomes.clone(),
                market,
            }
        } else {
            // let LOG = stub.offer_type == TotalGoals(Period::FirstHalf, Over(4)); //TODO
            // if LOG {
            //     trace!("reqs: {reqs:?}");
            // }

            // doesn't require player probabilities — can be explored as a whole
            let exploration = caching_context.explore(CacheableIntervalArgs {
                config: interval::Config {
                    intervals: self.config.intervals,
                    team_probs,
                    player_probs: sv![],
                    prune_thresholds,
                    expansions: reqs,
                },
                include_intervals: 0..self.config.intervals,
            });
            // if LOG {
            //     for prospect in &exploration.prospects {
            //         trace!("  prospect: {prospect:?}");
            //     }
            // }

            frame_prices_from_exploration(
                exploration,
                &stub.offer_type,
                stub.outcomes.items(),
                stub.normal,
                &stub.overround,
                price_bounds,
            )
        };
        Ok(offer)
    }

    pub fn derive_multi(
        &self,
        selections: &[(OfferType, Outcome)],
    ) -> Result<Timed<MultiDerivation>, MultiDerivationError> {
        if selections.is_empty() {
            return Err(MultiDerivationError::NoSelections(NoSelections));
        }
        const PRUNE_MIN_PROB: f64 = 0.0;//1e-5;

        Timed::result(|| {
            let mut caching_context = CachingContext::default();
            let mut agg_player_probs = FxHashMap::<Player, PlayerProbs>::with_capacity_and_hasher(
                interval::NUM_PLAYERS,
                Default::default(),
            );
            let mut agg_reqs = Expansions::empty();
            let mut sorted_selections = Vec::with_capacity(selections.len());

            let mut exploration_elapsed = Duration::new(0, 0);
            let mut query_elapsed = Duration::new(0, 0);
            for (offer_type, outcome) in selections {
                self.collect_requirements(
                    offer_type,
                    outcome,
                    &mut agg_reqs,
                    &mut agg_player_probs,
                )?;

                let offer =
                    self.offers
                        .get(offer_type)
                        .ok_or(MultiDerivationError::MissingDerivative(MissingDerivative {
                            offer_type: offer_type.clone(),
                            outcome: outcome.clone(),
                        }))?;
                let outcome_index = offer.outcomes.index_of(outcome).ok_or(
                    MultiDerivationError::MissingDerivative(MissingDerivative {
                        offer_type: offer_type.clone(),
                        outcome: outcome.clone(),
                    }),
                )?;
                let single_prob = offer.market.probs[outcome_index];
                // unrelated_prob *= single_prob;
                let single_price = offer.market.prices[outcome_index];
                let single_overround = 1.0 / single_prob / single_price;
                // overround *= 1.0 / single_prob / single_price;
                sorted_selections.push(Selection {
                    offer_type: offer_type.clone(),
                    outcome: outcome.clone(),
                    single_overround,
                    single_prob,
                });
            }
            trace!("agg_reqs: {agg_reqs:?}, agg_player_probs: {agg_player_probs:?}");
            sorted_selections.sort_by(|s1, s2| s1.single_prob.total_cmp(&s2.single_prob));
            trace!("sorted selections: {sorted_selections:?}");

            let FromIteratorResult(player_probs) = agg_player_probs
                .iter()
                .map(|(player, player_probs)| (player.clone(), player_probs.clone()))
                .collect();
            let player_probs = player_probs.map_err(|err| TooManyPlayers {
                capacity: err.capacity,
            })?;
            let team_probs = TeamProbs {
                h1_goals: self.goal_probs.clone().unwrap_or_default().h1,
                h2_goals: self.goal_probs.clone().unwrap_or_default().h2,
                assists: self.assist_probs.clone().unwrap_or_default(),
            };
            let prune_thresholds = PruneThresholds {
                max_total_goals: self.config.max_total_goals,
                min_prob: PRUNE_MIN_PROB,
            };
            let config = interval::Config {
                intervals: self.config.intervals,
                team_probs,
                player_probs,
                prune_thresholds,
                expansions: agg_reqs.clone(),
            };

            let pruned;
            let mut keep = Vec::with_capacity(sorted_selections.len());
            let mut redundancies = vec![];
            let mut lowest_prob = f64::MAX;
            {
                let exploration_start = Instant::now();
                let exploration = caching_context.explore(CacheableIntervalArgs {
                    config,
                    include_intervals: 0..self.config.intervals,
                });
                exploration_elapsed += exploration_start.elapsed();
                trace!("prospects: {}, took: {:?}", exploration.prospects.len(), exploration_start.elapsed());
                pruned = exploration.pruned;
                let query_start = Instant::now();
                for end_index in 1..=sorted_selections.len() {
                    let prefix = sorted_selections[0..end_index]
                        .iter()
                        .map(|selection| (selection.offer_type.clone(), selection.outcome.clone()))
                        .collect::<Vec<_>>();
                    let prob =
                        query::isolate_set(&prefix, &exploration.prospects, &exploration.player_lookup);
                    trace!("prefix: {prefix:?}, prob: {prob:.3}");
                    let tail = &sorted_selections[end_index - 1];
                    if prob < lowest_prob {
                        lowest_prob = prob;
                        keep.push(tail.clone());
                    } else {
                        redundancies.push(tail.clone());
                    }
                }
                query_elapsed += query_start.elapsed();
            }

            let mut fringes =
                FxHashMap::with_capacity_and_hasher(self.offers.len(), Default::default());

            // fringes
            for (offer_type, offer) in &self.offers {
                if !is_fringe_supported(offer_type) && selections_contain_offer_type(selections, offer_type) {
                    continue;
                }
                let reqs = requirements(offer_type);
                let reuse_exploration = !reqs.requires_player_goal_probs() && !reqs.requires_player_assist_probs();
                let mut exploration = None;
                let mut fringes_vec = Vec::with_capacity(offer.outcomes.len());
                for (outcome_index, outcome) in offer.outcomes.items().iter().enumerate() {
                    // let LOG = offer_type == &TotalGoals(Period::FirstHalf, Over(4)) && outcome == &Outcome::Over(4);

                    let single_prob = offer.market.probs[outcome_index];
                    if single_prob == 0.0 {
                        continue;
                    }
                    if selections_contains(selections, offer_type, outcome) {
                        continue;
                    }

                    let single_price = offer.market.prices[outcome_index];
                    let mut fringe_sorted_selections = keep.clone();
                    let single_overround = 1.0 / single_prob / single_price;

                    // if LOG {
                    //     trace!("price: {single_price:.3}, prob: {single_prob:.3}, overround: {single_overround:.3}");
                    // }
                    fringe_sorted_selections.push(Selection {
                        offer_type: offer_type.clone(),
                        outcome: outcome.clone(),
                        single_overround,
                        single_prob,
                    });
                    fringe_sorted_selections.sort_by(|s1, s2| s1.single_prob.total_cmp(&s2.single_prob));

                    let mut fringe_agg_reqs = agg_reqs.clone();
                    let mut fringe_agg_player_probs = agg_player_probs.clone();
                    self.collect_requirements(
                        offer_type,
                        outcome,
                        &mut fringe_agg_reqs,
                        &mut fringe_agg_player_probs,
                    )?;

                    // if LOG {
                    //     trace!("fringe_agg_reqs: {fringe_agg_reqs:?}, agg_player_probs: {fringe_agg_player_probs:?}");
                    //     trace!("fringe_sorted selections: {fringe_sorted_selections:?}");
                    // }

                    let FromIteratorResult(player_probs) = fringe_agg_player_probs
                        .iter()
                        .map(|(player, player_probs)| (player.clone(), player_probs.clone()))
                        .collect();
                    let player_probs = match player_probs {
                        Ok(player_probs) => player_probs,
                        Err(_) => {
                            trace!("skipping {offer_type:?}/{outcome:?}: too many players");
                            continue;
                        }
                    };
                    let team_probs = TeamProbs {
                        h1_goals: self.goal_probs.clone().unwrap_or_default().h1,
                        h2_goals: self.goal_probs.clone().unwrap_or_default().h2,
                        assists: self.assist_probs.clone().unwrap_or_default(),
                    };
                    let prune_thresholds = PruneThresholds {
                        max_total_goals: self.config.max_total_goals,
                        min_prob: PRUNE_MIN_PROB,
                    };
                    let config = interval::Config {
                        intervals: self.config.intervals,
                        team_probs,
                        player_probs,
                        prune_thresholds,
                        expansions: fringe_agg_reqs.clone(),
                    };

                    if exploration.is_none() || !reuse_exploration {
                        let exploration_start = Instant::now();
                        exploration = Some(caching_context.explore(CacheableIntervalArgs {
                            config,
                            include_intervals: 0..self.config.intervals,
                        }));
                        exploration_elapsed += exploration_start.elapsed();
                        trace!("fringe {offer_type:?}/{outcome:?}, prospects: {}, took {:?}", exploration.unwrap().prospects.len(), exploration_start.elapsed());
                    }
                    let exploration = exploration.unwrap();
                    let fringe_pruned = exploration.pruned;

                    let mut fringe_keep = Vec::with_capacity(fringe_sorted_selections.len());
                    let mut fringe_redundancies = vec![];
                    let mut fringe_lowest_prob = f64::MAX;

                    let query_start = Instant::now();
                    for end_index in 1..=fringe_sorted_selections.len() {
                        let prefix = fringe_sorted_selections[0..end_index]
                            .iter()
                            .map(|selection| (selection.offer_type.clone(), selection.outcome.clone()))
                            .collect::<Vec<_>>();
                        let prob =
                            query::isolate_set(&prefix, &exploration.prospects, &exploration.player_lookup);
                        // debug!("prefix: {prefix:?}, prob: {prob:.3}");
                        let tail = &fringe_sorted_selections[end_index - 1];
                        if prob < fringe_lowest_prob {
                            fringe_lowest_prob = prob;
                            fringe_keep.push(tail.clone());
                        } else {
                            fringe_redundancies.push(tail.clone());
                        }
                    }
                    query_elapsed += query_start.elapsed();

                    let probability = fringe_lowest_prob / (1.0 - fringe_pruned);
                    let overround = fringe_keep
                        .iter()
                        .map(|selection| selection.single_overround)
                        .product::<f64>();
                    let price = 1.0 / probability / overround;
                    let unrelated_prob = fringe_keep.iter().map(|selection| selection.single_prob).product::<f64>();
                    let relatedness = unrelated_prob / probability;
                    let redundancies = fringe_redundancies
                        .into_iter()
                        .map(|selection| (selection.offer_type, selection.outcome))
                        .collect::<Vec<_>>();

                    // if LOG {
                    //     trace!("probability: {probability:.3}, overround: {overround:.3}, price: {price:.3}, pruned: {:.3}", exploration.pruned);
                    //     // for prospect in &exploration.prospects {
                    //     //     trace!("  prospect: {prospect:?}");
                    //     // }
                    // }
                    fringes_vec.push(Fringe {
                        outcome: outcome.clone(),
                        quotation: DerivedPrice {
                            probability,
                            price,
                        },
                        redundancies,
                        relatedness,
                    });
                }

                fringes.insert(offer_type.clone(), fringes_vec);
            }

            debug!("pruned: {:.6}", pruned);
            debug!("cache stats: {:?}", caching_context.stats);
            debug!("elapsed: exploration: {exploration_elapsed:?}, query: {query_elapsed:?}");

            let probability = lowest_prob / (1.0 - pruned);
            let overround = keep
                .iter()
                .map(|selection| selection.single_overround)
                .product::<f64>();
            let price = 1.0 / probability / overround;
            let unrelated_prob = keep.iter().map(|selection| selection.single_prob).product::<f64>();
            let relatedness = unrelated_prob / probability;
            let redundancies = redundancies
                .into_iter()
                .map(|selection| (selection.offer_type, selection.outcome))
                .collect::<Vec<_>>();
            Ok(MultiDerivation {
                quotation: DerivedPrice {
                    probability,
                    price,
                },
                redundancies,
                relatedness,
                fringes,
            })
        })
    }

    fn collect_requirements(
        &self,
        offer_type: &OfferType,
        outcome: &Outcome,
        agg_reqs: &mut Expansions,
        agg_player_probs: &mut FxHashMap<Player, PlayerProbs>,
    ) -> Result<(), MultiDerivationError> {
        offer_type.validate_outcome(outcome)?;
        let reqs = requirements(offer_type);
        let requires_player_goal_probs = reqs.requires_player_goal_probs();
        let requires_player_assist_probs = reqs.requires_player_assist_probs();
        if requires_player_goal_probs || requires_player_assist_probs {
            match outcome.get_player() {
                None => {}
                Some(player) => {
                    let player_probs = get_or_create_player(agg_player_probs, player.clone());
                    if requires_player_goal_probs {
                        let player_goal_prob = self.require_player_goal_prob(player)?;
                        player_probs.goal = Some(player_goal_prob);
                    }
                    if requires_player_assist_probs {
                        let player_assist_prob = self.require_player_assist_prob(player)?;
                        player_probs.assist = Some(player_assist_prob);
                    }
                }
            }
        }
        *agg_reqs += reqs;
        Ok(())
    }

    fn ensure_team_requirements(&self, reqs: &Expansions) -> Result<(), UnmetRequirement> {
        if reqs.requires_team_goal_probs() {
            self.require_team_goal_probs()?;
        }
        if reqs.requires_team_assist_probs() {
            self.require_team_assist_probs()?;
        }
        Ok(())
    }

    fn require_team_goal_probs(&self) -> Result<&GoalProbs, UnmetRequirement> {
        self.goal_probs
            .as_ref()
            .ok_or(UnmetRequirement::TeamGoalProbabilities)
    }

    fn require_team_assist_probs(&self) -> Result<&UnivariateProbs, UnmetRequirement> {
        self.assist_probs
            .as_ref()
            .ok_or(UnmetRequirement::TeamAssistProbabilities)
    }

    fn require_player_goal_prob(&self, player: &Player) -> Result<f64, UnmetRequirement> {
        self.player_probs
            .get(player)
            .and_then(|player_probs| player_probs.goal)
            .ok_or_else(|| UnmetRequirement::PlayerGoalProbability(player.clone()))
    }

    fn require_player_assist_prob(&self, player: &Player) -> Result<f64, UnmetRequirement> {
        self.player_probs
            .get(player)
            .and_then(|player_probs| player_probs.assist)
            .ok_or_else(|| UnmetRequirement::PlayerAssistProbability(player.clone()))
    }
}

fn is_fringe_supported(offer_type: &OfferType) -> bool {
    matches!(offer_type, OfferType::AnytimeAssist | OfferType::AnytimeGoalscorer)
}

fn selections_contain_offer_type(selections: &[(OfferType, Outcome)], search_offer_type: &OfferType) -> bool {
    selections.iter().any(|(offer_type, _)| offer_type == search_offer_type)
}

fn selections_contains(selections: &[(OfferType, Outcome)], search_offer_type: &OfferType, search_outcome: &Outcome) -> bool {
    selections.iter().any(|(offer_type, outcome)| offer_type == search_offer_type && outcome == search_outcome)
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
    outcomes: &[Outcome],
    normal: f64,
    overround: &Overround,
    price_bounds: &PriceBounds,
) -> Offer {
    let mut probs = outcomes
        .iter()
        .map(|outcome| {
            isolate(
                offer_type,
                outcome,
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
