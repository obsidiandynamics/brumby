use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;

use crate::entity::{MarketType, OutcomeType, Player, Score, Side};
use crate::scoregrid::GoalEvent;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Prospect {
    pub score: Score,
    pub stats: Vec<PlayerStats>,
    pub first_scorer: Option<usize>,
}
impl Prospect {
    fn init(players: usize) -> Prospect {
        let stats = vec![PlayerStats::default(); players];
        Prospect {
            score: Score { home: 0, away: 0 },
            stats,
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
    pub players: Vec<(Player, f64)>,
}

#[derive(Debug)]
pub struct Exploration {
    pub player_lookup: Vec<Player>,
    pub prospects: Prospects,
    pub pruned: f64,
}

#[derive(Debug)]
struct PartialProspect<'a> {
    home_scorer: Option<usize>,
    away_scorer: Option<usize>,
    first_scoring_side: Option<&'a Side>,
    prob: f64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PlayerStats {
    pub goals: u8,
}

pub fn explore(config: &IntervalConfig) -> Exploration {
    let mut player_lookup = Vec::with_capacity(config.players.len() + 1);
    let mut home_scorers = Vec::with_capacity(config.players.len() + 1);
    let mut away_scorers = Vec::with_capacity(config.players.len() + 1);
    let mut combined_home_scorer_prob = 0.0;
    let mut combined_away_scorer_prob = 0.0;
    for (player_index, (player, goal_prob)) in config.players.iter().enumerate() {
        player_lookup.push(player.clone());
        match player {
            Player::Named(side, _) => match side {
                Side::Home => {
                    combined_home_scorer_prob += goal_prob;
                    home_scorers.push((player_index, *goal_prob));
                }
                Side::Away => {
                    combined_away_scorer_prob += goal_prob;
                    away_scorers.push((player_index, *goal_prob));
                }
            },
            Player::Other => panic!("unsupported scorer {:?}", Player::Other),
        }
    }
    player_lookup.push(Player::Other);
    home_scorers.push((config.players.len(), 1.0 - combined_home_scorer_prob));
    away_scorers.push((config.players.len(), 1.0 - combined_away_scorer_prob));

    let mut current_prospects = init_prospects(1);
    current_prospects.insert(Prospect::init(player_lookup.len()), 1.0);
    let neither_prob = 1.0 - config.home_prob - config.away_prob - config.common_prob;
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
                    for (player_index, player_prob) in &home_scorers {
                        let partial = PartialProspect {
                            home_scorer: Some(*player_index),
                            away_scorer: None,
                            first_scoring_side: Some(&Side::Home),
                            prob: config.home_prob * player_prob,
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Away => {
                    for (player_index, player_prob) in &away_scorers {
                        let partial = PartialProspect {
                            home_scorer: None,
                            away_scorer: Some(*player_index),
                            first_scoring_side: Some(&Side::Away),
                            prob: config.away_prob * player_prob,
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Both => {
                    for (home_player_index, home_player_prob) in &home_scorers {
                        for (away_player_index, away_player_prob) in &away_scorers {
                            for first_scoring_side in [&Side::Home, &Side::Away] {
                                let partial = PartialProspect {
                                    home_scorer: Some(*home_player_index),
                                    away_scorer: Some(*away_player_index),
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
        player_lookup,
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
        let partial_goals = if partial.home_scorer.is_some() { 1 } else { 0 }
            + if partial.away_scorer.is_some() { 1 } else { 0 };
        // let partial_goals = partial.home_scorer.map(|_| 1).unwrap_or(0)
        //     + partial.away_scorer.map(|_| 1).unwrap_or(0);
        if current.score.total() + partial_goals > config.max_total_goals {
            pruned += merged_prob;
            continue;
        }

        let mut merged = current.clone();
        if let Some(scorer) = partial.home_scorer {
            merged.stats[scorer].goals += 1;
            merged.score.home += 1;
            if merged.first_scorer.is_none() && partial.first_scoring_side.unwrap() == &Side::Home {
                merged.first_scorer = Some(scorer);
            }
        }
        if let Some(scorer) = partial.away_scorer {
            merged.stats[scorer].goals += 1;
            merged.score.away += 1;
            if merged.first_scorer.is_none() && partial.first_scoring_side.unwrap() == &Side::Away {
                merged.first_scorer = Some(scorer);
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
pub fn isolate(
    market_type: &MarketType,
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &[Player],
) -> f64 {
    match market_type {
        MarketType::HeadToHead => unimplemented!(),
        MarketType::TotalGoalsOverUnder(_) => unimplemented!(),
        MarketType::CorrectScore => unimplemented!(),
        MarketType::DrawNoBet => unimplemented!(),
        MarketType::AnytimeGoalscorer => {
            isolate_anytime_goalscorer(outcome_type, prospects, player_lookup)
        }
        MarketType::FirstGoalscorer => {
            isolate_first_goalscorer(outcome_type, prospects, player_lookup)
        }
        MarketType::PlayerShotsOnTarget(_) => unimplemented!(),
        MarketType::AnytimeAssist => unimplemented!(),
    }
}

#[must_use]
fn isolate_first_goalscorer(
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &[Player],
) -> f64 {
    match outcome_type {
        OutcomeType::Player(player) => prospects
            .iter()
            .filter(|(prospect, _)| {
                prospect
                    .first_scorer
                    .map(|scorer| &player_lookup[scorer] == player)
                    .unwrap_or(false)
                // prospect.first_scorer.filter(|scorer| &player_lookup[*scorer] == player).iter().count() == 1
            })
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
fn isolate_anytime_goalscorer(
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &[Player],
) -> f64 {
    match outcome_type {
        OutcomeType::Player(player) => prospects
            .iter()
            .filter(|(prospect, _)| {
                let scorer = prospect
                    .stats
                    .iter()
                    .enumerate()
                    .find(|(scorer_index, _)| &player_lookup[*scorer_index] == player);
                match scorer {
                    None => {
                        panic!("missing {player:?} from stats")
                    }
                    Some((_, stats)) => stats.goals > 0,
                }
            })
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
