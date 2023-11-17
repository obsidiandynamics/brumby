use std::collections::BTreeMap;

use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;

use crate::entity::{MarketType, OutcomeType, Player, Score, Side};
use crate::scoregrid::GoalEvent;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Prospect {
    pub score: Score,
    pub scorers: BTreeMap<Player, u8>,
    pub first_scorer: Option<Player>,
}

impl Default for Prospect {
    fn default() -> Self {
        Self {
            score: Score { home: 0, away: 0 },
            scorers: Default::default(),
            first_scorer: None,
        }
    }
}

pub type Prospects = FxHashMap<Prospect, f64>;

pub fn init_prospects(capacity: usize) -> Prospects {
    Prospects::with_capacity_and_hasher(capacity, Default::default())
}

#[derive(Debug)]
pub struct IntervalConfig {
    pub intervals: u8,
    pub home_prob: f64,
    pub away_prob: f64,
    pub common_prob: f64,
    pub max_total_goals: u16,
    pub scorers: Vec<(Player, f64)>,
}

#[derive(Debug)]
pub struct Exploration {
    pub prospects: Prospects,
    pub pruned: f64,
}

#[derive(Debug)]
struct PartialProspect<'a> {
    home_scorer: Option<&'a Player>,
    away_scorer: Option<&'a Player>,
    first_scoring_side: Option<&'a Side>,
    prob: f64,
}

pub fn explore(config: &IntervalConfig) -> Exploration {
    let mut current_prospects = init_prospects(1);
    current_prospects.insert(Prospect::default(), 1.0);
    let neither_prob = 1.0 - config.home_prob - config.away_prob - config.common_prob;
    let mut home_scorers = Vec::with_capacity(config.scorers.len() + 1);
    let mut away_scorers = Vec::with_capacity(config.scorers.len() + 1);
    let mut combined_home_scorer_prob = 0.0;
    let mut combined_away_scorer_prob = 0.0;
    for (player, prob) in &config.scorers {
        match player {
            Player::Named(side, _) => match side {
                Side::Home => {
                    combined_home_scorer_prob += prob;
                    home_scorers.push((player, *prob));
                }
                Side::Away => {
                    combined_away_scorer_prob += prob;
                    away_scorers.push((player, *prob));
                }
            },
            Player::Other => panic!("unsupported scorer {:?}", Player::Other),
        }
    }
    home_scorers.push((&Player::Other, 1.0 - combined_home_scorer_prob));
    away_scorers.push((&Player::Other, 1.0 - combined_away_scorer_prob));

    let mut pruned = 0.0;

    for _ in 0..config.intervals {
        let mut next_prospects = init_prospects(current_prospects.len() * 4);
        for goal_event in GoalEvent::iter() {
            match goal_event {
                GoalEvent::Neither => {
                    let partial = PartialProspect {
                        home_scorer: None,
                        away_scorer: None,
                        first_scoring_side: None,
                        prob: neither_prob,
                    };
                    pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                }
                GoalEvent::Home => {
                    for (player, player_prob) in &home_scorers {
                        let partial = PartialProspect {
                            home_scorer: Some(player),
                            away_scorer: None,
                            first_scoring_side: Some(&Side::Home),
                            prob: config.home_prob * player_prob,
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Away => {
                    for (player, player_prob) in &away_scorers {
                        let partial = PartialProspect {
                            home_scorer: None,
                            away_scorer: Some(player),
                            first_scoring_side: Some(&Side::Away),
                            prob: config.away_prob * player_prob,
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Both => {
                    for (home_player, home_player_prob) in &home_scorers {
                        for (away_player, away_player_prob) in &away_scorers {
                            for first_scoring_side in [&Side::Home, &Side::Away] {
                                let partial = PartialProspect {
                                    home_scorer: Some(home_player),
                                    away_scorer: Some(away_player),
                                    first_scoring_side: Some(first_scoring_side),
                                    prob: config.common_prob
                                        * home_player_prob
                                        * away_player_prob
                                        * 0.5,
                                };
                                pruned +=
                                    merge(config, &current_prospects, partial, &mut next_prospects);
                            }
                        }
                    }
                }
            };
        }
        current_prospects = next_prospects;
    }

    Exploration {
        prospects: current_prospects,
        pruned,
    }
}

#[must_use]
fn merge(
    config: &IntervalConfig,
    current_prospects: &Prospects,
    partial: PartialProspect,
    next_prospects: &mut Prospects,
) -> f64 {
    let mut pruned = 0.0;
    for (current, current_prob) in current_prospects {
        let merged_prob = *current_prob * partial.prob;
        let partial_goals = partial.home_scorer.map(|_| 1).unwrap_or(0)
            + partial.away_scorer.map(|_| 1).unwrap_or(0);
        if current.score.total() + partial_goals > config.max_total_goals {
            pruned += merged_prob;
            continue;
        }

        let mut merged = current.clone();
        if let Some(scorer) = partial.home_scorer {
            merged
                .scorers
                .entry(scorer.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            merged.score.home += 1;
            if merged.first_scorer.is_none() && partial.first_scoring_side.unwrap() == &Side::Home {
                merged.first_scorer = Some(scorer.clone());
            }
        }
        if let Some(scorer) = partial.away_scorer {
            merged
                .scorers
                .entry(scorer.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            merged.score.away += 1;
            if merged.first_scorer.is_none() && partial.first_scoring_side.unwrap() == &Side::Away {
                merged.first_scorer = Some(scorer.clone());
            }
        }
        next_prospects
            .entry(merged)
            .and_modify(|prob| *prob += merged_prob)
            .or_insert(merged_prob);
    }
    pruned
}

#[must_use]
pub fn isolate(market_type: &MarketType, outcome_type: &OutcomeType, prospects: &Prospects) -> f64 {
    match market_type {
        MarketType::HeadToHead => unimplemented!(),
        MarketType::TotalGoalsOverUnder(_) => unimplemented!(),
        MarketType::CorrectScore => unimplemented!(),
        MarketType::DrawNoBet => unimplemented!(),
        MarketType::AnytimeGoalscorer => isolate_anytime_goalscorer(outcome_type, prospects),
        MarketType::FirstGoalscorer => isolate_first_goalscorer(outcome_type, prospects),
        MarketType::PlayerShotsOnTarget(_) => unimplemented!(),
        MarketType::AnytimeAssist => unimplemented!(),
    }
}

#[must_use]
fn isolate_first_goalscorer(outcome_type: &OutcomeType, prospects: &Prospects) -> f64 {
    match outcome_type {
        OutcomeType::Player(player) => prospects
            .iter()
            .filter(|(prospect, _)| prospect.first_scorer.as_ref() == Some(player))
            .map(|(_, prob)| prob)
            .sum(),
        OutcomeType::None => prospects
            .iter()
            .filter(|(prospect, _)| prospect.first_scorer.is_none())
            .map(|(_, prob)| prob)
            .sum(),
        _ => panic!(
            "{outcome_type:?} unsupported in {:?}",
            MarketType::FirstGoalscorer
        ),
    }
}

#[must_use]
fn isolate_anytime_goalscorer(outcome_type: &OutcomeType, prospects: &Prospects) -> f64 {
    match outcome_type {
        OutcomeType::Player(player) => prospects
            .iter()
            .filter(|(prospect, _)| prospect.scorers.contains_key(player))
            .map(|(_, prob)| prob)
            .sum(),
        OutcomeType::None => prospects
            .iter()
            .filter(|(prospect, _)| prospect.first_scorer.is_none())
            .map(|(_, prob)| prob)
            .sum(),
        _ => panic!(
            "{outcome_type:?} unsupported in {:?}",
            MarketType::AnytimeGoalscorer
        ),
    }
}

#[cfg(test)]
mod tests;
