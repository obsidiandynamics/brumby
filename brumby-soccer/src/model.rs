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

use crate::domain::validation::{InvalidOffer, InvalidOfferType, InvalidOutcome, MissingOutcome, UnvalidatedOffer};
use crate::domain::{
    DrawHandicap, Offer, OfferCategory, OfferType, Outcome, Over, Period, Player, Side, WinHandicap,
};
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

    #[error("{0}")]
    MissingOffer(#[from] MissingOffer),

    #[error("{0}")]
    InvalidOfferType(#[from] InvalidOfferType),
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

    #[error("{0}")]
    AuxiliaryOffer(#[from] AuxiliaryOffer),

    #[error("{0}")]
    InvalidOfferType(#[from] InvalidOfferType),
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

#[derive(Debug, Error, PartialEq, Eq)]
#[error("auxiliary {offer_type:?}")]
pub struct AuxiliaryOffer {
    pub offer_type: OfferType,
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
struct DetailedSelection {
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

    pub fn insert_offer(&mut self, offer: Offer) {
        self.offers.insert(offer.offer_type.clone(), offer);
    }

    fn get_offer(&self, offer_type: &OfferType) -> Result<&Offer, MissingOffer> {
        self.offers
            .get(offer_type)
            .ok_or(MissingOffer::Type(offer_type.clone()))
    }

    pub fn derive(
        &mut self,
        stubs: &[Stub],
        price_bounds: &PriceBounds,
    ) -> Result<Timed<CacheStats>, SingleDerivationError> {
        Timed::result(|| {
            let mut caching_context = CachingContext::default();
            let mut auxiliary_stubs = vec![];
            for stub in stubs {
                stub.offer_type.validate()?;
                stub.offer_type.validate_outcomes(&stub.outcomes)?;
                if stub.offer_type.is_auxiliary() {
                    auxiliary_stubs.push(stub);
                } else {
                    debug!(
                        "deriving {:?} ({} outcomes)",
                        stub.offer_type,
                        stub.outcomes.len()
                    );
                    let start = Instant::now();
                    let offer = self.derive_offer(stub, price_bounds, &mut caching_context)?;
                    debug!(
                        "... took {:?}, progressive {:?}",
                        start.elapsed(),
                        caching_context.stats
                    );
                    self.insert_offer(offer);
                }
            }
            for stub in auxiliary_stubs {
                debug!(
                    "deriving auxiliary {:?} ({} outcomes)",
                    stub.offer_type,
                    stub.outcomes.len()
                );
                let offer = match stub.offer_type {
                    OfferType::DrawNoBet(_) => self.derive_draw_no_bet(stub, price_bounds)?,
                    OfferType::SplitHandicap(_, _, _) => {
                        self.derive_split_handicap(stub, price_bounds)?
                    }
                    _ => unreachable!(),
                };
                self.insert_offer(offer);
            }
            Ok(caching_context.stats)
        })
    }

    #[inline(always)]
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
            // let LOG = stub.offer_type == OfferType::TotalGoals(Period::FullTime, Over(5)); //TODO
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

            // if LOG {
            //     trace!("{:?}", frame_prices_from_exploration(
            //         exploration,
            //         &stub.offer_type,
            //         stub.outcomes.items(),
            //         stub.normal,
            //         &stub.overround,
            //         price_bounds,
            //     ));
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

    #[inline(always)]
    fn derive_draw_no_bet(
        &mut self,
        stub: &Stub,
        price_bounds: &PriceBounds,
    ) -> Result<Offer, SingleDerivationError> {
        let draw_handicap = match stub.offer_type {
            OfferType::DrawNoBet(ref draw_handicap) => draw_handicap,
            _ => unreachable!(),
        };

        let source_offer_type = OfferType::HeadToHead(Period::FullTime, draw_handicap.clone());
        let source_offer = self.get_offer(&source_offer_type)?;

        let home_outcome = Outcome::Win(Side::Home, draw_handicap.to_win_handicap());
        let home_prob = source_offer.get_probability(&home_outcome).unwrap();
        let away_outcome =
            Outcome::Win(Side::Away, draw_handicap.to_win_handicap().flip_european());
        let away_prob = source_offer.get_probability(&away_outcome).unwrap();

        let mut probs = [home_prob, away_prob];
        probs.normalise(stub.normal);
        // trace!("DNB probs: {probs:?}, sum: {:.6}", probs.sum());
        let market = Market::frame(&stub.overround, probs.to_vec(), price_bounds);
        let outcomes = [home_outcome, away_outcome];
        Ok(Offer {
            offer_type: stub.offer_type.clone(),
            outcomes: HashLookup::from(outcomes.to_vec()),
            market,
        })
    }

    #[inline(always)]
    fn derive_split_handicap(
        &mut self,
        stub: &Stub,
        price_bounds: &PriceBounds,
    ) -> Result<Offer, SingleDerivationError> {
        let (period, draw_handicap, win_handicap) = match stub.offer_type {
            OfferType::SplitHandicap(ref period, ref draw_handicap, ref win_handicap) => {
                (period, draw_handicap, win_handicap)
            }
            _ => unreachable!(),
        };

        let euro_offer_type = OfferType::HeadToHead(period.clone(), draw_handicap.clone());
        let euro_offer = self.get_offer(&euro_offer_type)?;
        let asian_offer_type = OfferType::AsianHandicap(period.clone(), win_handicap.clone());
        let asian_offer = self.get_offer(&asian_offer_type)?;

        let draw_prob = euro_offer
            .get_probability(&Outcome::Draw(draw_handicap.clone()))
            .unwrap();
        let (home_prob, away_prob) = match (draw_handicap, win_handicap) {
            (DrawHandicap::Ahead(ahead), WinHandicap::AheadOver(ahead_over)) => {
                if ahead == ahead_over {
                    // -x.25 case
                    let asian_win_prob = asian_offer
                        .get_probability(&Outcome::Win(Side::Home, win_handicap.clone()))
                        .unwrap();
                    let home_prob = asian_win_prob / (1.0 - 0.5 * draw_prob);
                    (home_prob, 1.0 - home_prob)
                } else {
                    // -x.75 case
                    assert_eq!(*ahead, ahead_over + 1);
                    let euro_win_prob = euro_offer
                        .get_probability(&Outcome::Win(Side::Home, draw_handicap.to_win_handicap()))
                        .unwrap();
                    let home_prob = (euro_win_prob + 0.5 * draw_prob) / (1.0 - 0.5 * draw_prob);
                    (home_prob, 1.0 - home_prob)
                }
            }
            (_, WinHandicap::BehindUnder(behind_under)) => {
                let behind = match draw_handicap {
                    DrawHandicap::Ahead(0) => 0,    // Behind(0) is always written as Ahead(0) by convention
                    DrawHandicap::Behind(by) => *by,
                    _ => unreachable!()
                };
                if behind == *behind_under {
                    // +x.75 case
                    let euro_win_prob = euro_offer
                        .get_probability(&Outcome::Win(Side::Away, draw_handicap.to_win_handicap().flip_european()))
                        .unwrap();
                    let away_prob = (euro_win_prob + 0.5 * draw_prob) / (1.0 - 0.5 * draw_prob);
                    (1.0 - away_prob, away_prob)
                } else {
                    // +x.25 case
                    assert_eq!(behind + 1, *behind_under);
                    let asian_win_prob = asian_offer
                        .get_probability(&Outcome::Win(Side::Away, win_handicap.flip_asian()))
                        .unwrap();
                    let away_prob = asian_win_prob / (1.0 - 0.5 * draw_prob);
                    (1.0 - away_prob, away_prob)
                }
            }
            _ => unreachable!(),
        };

        let home_outcome = Outcome::SplitWin(Side::Home, draw_handicap.clone(), win_handicap.clone());
        let away_outcome = Outcome::SplitWin(Side::Away, draw_handicap.flip(), win_handicap.flip_asian());
        let mut probs = [home_prob, away_prob];
        probs.normalise(stub.normal);
        // trace!("DNB probs: {probs:?}, sum: {:.6}", probs.sum());
        let market = Market::frame(&stub.overround, probs.to_vec(), price_bounds);
        let outcomes = [home_outcome, away_outcome];
        Ok(Offer {
            offer_type: stub.offer_type.clone(),
            outcomes: HashLookup::from(outcomes.to_vec()),
            market,
        })
    }

    pub fn derive_multi(
        &self,
        selections: &[(OfferType, Outcome)],
    ) -> Result<Timed<MultiDerivation>, MultiDerivationError> {
        if selections.is_empty() {
            return Err(MultiDerivationError::NoSelections(NoSelections));
        }
        const PRUNE_MIN_PROB: f64 = 1e-4;

        #[inline(always)]
        fn product_of_overrounds(selections: &[DetailedSelection]) -> f64 {
            selections
                .iter()
                .map(|selection| selection.single_overround)
                .product()
        }

        #[inline(always)]
        fn product_of_unrelated_probs(selections: &[DetailedSelection]) -> f64 {
            selections
                .iter()
                .map(|selection| selection.single_prob)
                .product()
        }

        #[inline(always)]
        fn resolve_selection(
            offer: &Offer,
            outcome: &Outcome,
            outcome_index: usize,
        ) -> DetailedSelection {
            let single_prob = offer.market.probs[outcome_index];
            let single_price = offer.market.prices[outcome_index];
            let single_overround = 1.0 / single_prob / single_price;
            DetailedSelection {
                offer_type: offer.offer_type.clone(),
                outcome: outcome.clone(),
                single_overround,
                single_prob,
            }
        }

        #[inline(always)]
        fn strip_details(selections: Vec<DetailedSelection>) -> Vec<(OfferType, Outcome)> {
            selections
                .into_iter()
                .map(|selection| (selection.offer_type, selection.outcome))
                .collect()
        }

        #[inline(always)]
        fn sort_selections_by_increasing_prob(selections: &mut [DetailedSelection]) {
            selections.sort_by(|s1, s2| s1.single_prob.total_cmp(&s2.single_prob))
        }

        struct ScanPrefixResult {
            keep: Vec<DetailedSelection>,
            drop: Vec<DetailedSelection>,
            lowest_prob: f64,
        }

        #[inline(always)]
        fn scan_prefix(
            sorted_selections: &[DetailedSelection],
            exploration: &Exploration,
        ) -> ScanPrefixResult {
            let mut keep = Vec::with_capacity(sorted_selections.len());
            let mut drop = vec![];
            let mut lowest_prob = f64::MAX;

            for end_index in 1..=sorted_selections.len() {
                let prefix = sorted_selections[0..end_index]
                    .iter()
                    .map(|selection| (selection.offer_type.clone(), selection.outcome.clone()))
                    .collect::<Vec<_>>();
                let prob =
                    query::isolate_set(&prefix, &exploration.prospects, &exploration.player_lookup);
                // if LOG { trace!("fringe prefix: {prefix:?}, prob: {prob:.3}"); }
                let tail = &sorted_selections[end_index - 1];
                if prob < lowest_prob {
                    lowest_prob = prob;
                    keep.push(tail.clone());
                } else {
                    drop.push(tail.clone());
                }
            }

            ScanPrefixResult {
                keep,
                drop,
                lowest_prob,
            }
        }

        Timed::result(|| {
            let mut caching_context = CachingContext::default();
            let mut agg_player_probs = FxHashMap::<Player, PlayerProbs>::with_capacity_and_hasher(
                interval::NUM_PLAYERS,
                Default::default(),
            );
            let mut agg_reqs = Expansions::empty();
            let mut sorted_selections = Vec::with_capacity(selections.len());

            let mut exploration_elapsed = Duration::default();
            let mut query_elapsed = Duration::default();
            for (offer_type, outcome) in selections {
                if offer_type.is_auxiliary() {
                    return Err(MultiDerivationError::AuxiliaryOffer(AuxiliaryOffer {
                        offer_type: offer_type.clone(),
                    }));
                }

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
                let selection = resolve_selection(offer, outcome, outcome_index);
                sorted_selections.push(selection);
            }
            trace!("agg_reqs: {agg_reqs:?}, agg_player_probs: {agg_player_probs:?}");
            sort_selections_by_increasing_prob(&mut sorted_selections);
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
                team_probs: team_probs.clone(),
                player_probs,
                prune_thresholds: prune_thresholds.clone(),
                expansions: agg_reqs.clone(),
            };

            let exploration_start = Instant::now();
            let exploration = caching_context.explore(CacheableIntervalArgs {
                config,
                include_intervals: 0..self.config.intervals,
            });
            exploration_elapsed += exploration_start.elapsed();
            trace!(
                "prospects: {}, took: {:?}",
                exploration.prospects.len(),
                exploration_start.elapsed()
            );
            let pruned = exploration.pruned;
            let query_start = Instant::now();
            let scan_result = scan_prefix(&sorted_selections, exploration);
            query_elapsed += query_start.elapsed();

            let mut fringes =
                FxHashMap::with_capacity_and_hasher(self.offers.len(), Default::default());

            // fringes
            for (offer_type, offer) in &self.offers {
                if offer_type.is_auxiliary() {
                    continue;
                }

                if !is_fringe_supported(offer_type)
                    && selections_contain_offer_type(selections, offer_type)
                {
                    continue;
                }
                let reqs = requirements(offer_type);
                let reuse_exploration =
                    !reqs.requires_player_goal_probs() && !reqs.requires_player_assist_probs();
                let mut fringe_exploration = None;
                let mut fringes_vec = Vec::with_capacity(offer.outcomes.len());
                for (outcome_index, outcome) in offer.outcomes.items().iter().enumerate() {
                    // let LOG = offer_type == &TotalGoals(Period::FirstHalf, Over(4)) && outcome == &Outcome::Over(4);
                    // let LOG = offer_type == &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)) && outcome == &Outcome::Win(Side::Home, WinHandicap::AheadOver(0));
                    if selections_contains(selections, offer_type, outcome) {
                        continue;
                    }

                    let selection = resolve_selection(offer, outcome, outcome_index);
                    if selection.single_prob == 0.0 {
                        continue;
                    }

                    // if LOG {
                    //     trace!("price: {single_price:.3}, prob: {single_prob:.3}, overround: {single_overround:.3}");
                    // }
                    let mut fringe_sorted_selections =
                        Vec::with_capacity(scan_result.keep.len() + 1);
                    fringe_sorted_selections.clone_from(&scan_result.keep);
                    fringe_sorted_selections.push(selection);
                    sort_selections_by_increasing_prob(&mut fringe_sorted_selections);

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

                    let FromIteratorResult(fringe_player_probs) =
                        fringe_agg_player_probs.into_iter().collect();
                    let player_probs = match fringe_player_probs {
                        Ok(player_probs) => player_probs,
                        Err(_) => {
                            trace!("skipping {offer_type:?}/{outcome:?}: too many players");
                            continue;
                        }
                    };
                    let config = interval::Config {
                        intervals: self.config.intervals,
                        team_probs: team_probs.clone(),
                        player_probs,
                        prune_thresholds: prune_thresholds.clone(),
                        expansions: fringe_agg_reqs.clone(),
                    };

                    if fringe_exploration.is_none() || !reuse_exploration {
                        let exploration_start = Instant::now();
                        fringe_exploration = Some(caching_context.explore(CacheableIntervalArgs {
                            config,
                            include_intervals: 0..self.config.intervals,
                        }));
                        exploration_elapsed += exploration_start.elapsed();
                        trace!(
                            "fringe {offer_type:?}/{outcome:?}, prospects: {}, took {:?}",
                            fringe_exploration.unwrap().prospects.len(),
                            exploration_start.elapsed()
                        );
                    }
                    let fringe_exploration = fringe_exploration.unwrap();

                    let query_start = Instant::now();
                    let fringe_scan_result =
                        scan_prefix(&fringe_sorted_selections, fringe_exploration);
                    query_elapsed += query_start.elapsed();

                    let probability = fringe_scan_result.lowest_prob;
                    let overround = product_of_overrounds(&fringe_scan_result.keep);
                    let price = 1.0 / probability / overround;
                    let unrelated_prob = product_of_unrelated_probs(&fringe_scan_result.keep);
                    let relatedness = unrelated_prob / probability;
                    let redundancies = strip_details(fringe_scan_result.drop);

                    // if LOG {
                    //     trace!("probability: {probability:.3}, overround: {overround:.3}, price: {price:.3}, pruned: {:.3}", exploration.pruned);
                    //     // for prospect in &exploration.prospects {
                    //     //     trace!("  prospect: {prospect:?}");
                    //     // }
                    // }
                    fringes_vec.push(Fringe {
                        outcome: outcome.clone(),
                        quotation: DerivedPrice { probability, price },
                        redundancies,
                        relatedness,
                    });
                }

                fringes.insert(offer_type.clone(), fringes_vec);
            }

            debug!("pruned: {:.6}", pruned);
            debug!("cache stats: {:?}", caching_context.stats);
            debug!("elapsed: exploration: {exploration_elapsed:?}, query: {query_elapsed:?}");

            let probability = scan_result.lowest_prob;
            let overround = product_of_overrounds(&scan_result.keep);
            let price = 1.0 / probability / overround;
            let unrelated_prob = product_of_unrelated_probs(&scan_result.keep);
            let relatedness = unrelated_prob / probability;
            let redundancies = strip_details(scan_result.drop);
            Ok(MultiDerivation {
                quotation: DerivedPrice { probability, price },
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
        offer_type.validate()?;
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
    matches!(
        offer_type,
        OfferType::AnytimeAssist | OfferType::AnytimeGoalscorer
    )
}

fn selections_contain_offer_type(
    selections: &[(OfferType, Outcome)],
    search_offer_type: &OfferType,
) -> bool {
    selections
        .iter()
        .any(|(offer_type, _)| offer_type == search_offer_type)
}

fn selections_contains(
    selections: &[(OfferType, Outcome)],
    search_offer_type: &OfferType,
    search_outcome: &Outcome,
) -> bool {
    selections
        .iter()
        .any(|(offer_type, outcome)| offer_type == search_offer_type && outcome == search_outcome)
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
    // trace!("probs: {probs:?}, sum: {:.6}", probs.sum());
    probs.normalise(normal);
    // trace!("probs: {probs:?}, sum: {:.6}", probs.sum());
    let market = Market::frame(overround, probs, price_bounds);
    Offer {
        offer_type: offer_type.clone(),
        outcomes: HashLookup::from(outcomes.to_vec()),
        market,
    }
}

#[cfg(test)]
mod tests;
