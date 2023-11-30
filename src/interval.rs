use std::collections::BTreeMap;

use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;

use crate::scoregrid::{GoalEvent, Player, Score};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Prospect {
    pub score: Score,
    pub scorers: BTreeMap<Player, u8>
}

impl Default for Prospect {
    fn default() -> Self {
        Self {
            score: Score { home: 0, away: 0},
            scorers: Default::default()
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
    // pub to_score: Option<(Player, f64)>
    pub home_scorers: Vec<(Player, f64)>,
    pub away_scorers: Vec<(Player, f64)>
}

pub fn other_player() -> Vec<(Player, f64)> {
    vec![(Player::Other, 1.0)]
}

#[derive(Debug)]
pub struct Exploration {
    pub prospects: Prospects,
    pub pruned: f64
}

#[derive(Debug)]
struct PartialProspect<'a> {
    home_scorer: Option<&'a Player>,
    away_scorer: Option<&'a Player>,
    prob: f64,
}

pub fn explore_all(config: &IntervalConfig) -> Exploration {
    let mut current_prospects = init_prospects(1);
    current_prospects.insert(Prospect::default(), 1.0);
    let neither_prob = 1.0 - config.home_prob - config.away_prob - config.common_prob;
    // let home_other_prob = 1.0 - config.to_score.filter(|(player, prob)| matches!(player, Player::Named(Side::Home, _))).map(|(_, prob)| prob).unwrap_or(0.0);
    // let away_other_prob = 1.0 - config.to_score.filter(|(player, prob)| matches!(player, Player::Named(Side::Away, _))).map(|(_, prob)| prob).unwrap_or(0.0);
    let mut pruned = 0.0;
    // let home_scorers = match &config.to_score {
    //     None => {
    //         vec![(&Player::Other, 1.0)]
    //     },
    //     Some((player, prob)) => {
    //         match player {
    //             Player::Named(Side::Home, _) => {
    //                 vec![(player, *prob), (&Player::Other, 1.0 - *prob)]
    //             }
    //             _ => {
    //                 vec![(&Player::Other, 1.0)]
    //             }
    //         }
    //     }
    // };
    // let away_scorers = match &config.to_score {
    //     None => {
    //         vec![(&Player::Other, 1.0)]
    //     },
    //     Some((player, prob)) => {
    //         match player {
    //             Player::Named(Side::Away, _) => {
    //                 vec![(player, *prob), (&Player::Other, 1.0 - *prob)]
    //             }
    //             _ => {
    //                 vec![(&Player::Other, 1.0)]
    //             }
    //         }
    //     }
    // };

    for _ in 0..config.intervals {
        let mut next_prospects = init_prospects(current_prospects.len() * 4);
        for goal_event in GoalEvent::iter() {
            match goal_event {
                GoalEvent::Neither => {
                    let partial = PartialProspect {
                        home_scorer: None,
                        away_scorer: None,
                        prob: neither_prob
                    };
                    pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                }
                GoalEvent::Home => {
                    for (player, player_prob) in &config.home_scorers {
                        let partial = PartialProspect {
                            home_scorer: Some(player),
                            away_scorer: None,
                            prob: config.home_prob * player_prob
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Away => {
                    for (player, player_prob) in &config.away_scorers {
                        let partial = PartialProspect {
                            home_scorer: None,
                            away_scorer: Some(player),
                            prob: config.away_prob * player_prob
                        };
                        pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                    }
                }
                GoalEvent::Both => {
                    for (home_player, home_player_prob) in &config.home_scorers {
                        for (away_player, away_player_prob) in &config.away_scorers {
                            let partial = PartialProspect {
                                home_scorer: Some(home_player),
                                away_scorer: Some(away_player),
                                prob: config.common_prob * home_player_prob * away_player_prob
                            };
                            pruned += merge(config, &current_prospects, partial, &mut next_prospects);
                        }
                    }
                }
            };

            // for (current, current_prob) in &current_prospects {
            //     let merged_prob = current_prob * next_prob;
            //     if current.score.total() + next.score.total() > config.max_total_goals {
            //         pruned += merged_prob;
            //         continue;
            //     }
            //     let merged = Prospect {
            //         score: Score {
            //             home: current.score.home + next.score.home,
            //             away: current.score.away + next.score.away,
            //         },
            //     };
            //     next_prospects
            //         .entry(merged)
            //         .and_modify(|prob| *prob += merged_prob)
            //         .or_insert(merged_prob);
            // }
        }
        current_prospects = next_prospects;
    }

    Exploration {
        prospects: current_prospects,
        pruned
    }
}

#[must_use]
fn merge(config: &IntervalConfig, current_prospects: &Prospects, partial: PartialProspect, next_prospects: &mut Prospects) -> f64 {
    let mut pruned = 0.0;
    for (current, current_prob) in current_prospects {
        let merged_prob = *current_prob * partial.prob;
        let partial_goals = partial.home_scorer.map(|_| 1).unwrap_or(0) + partial.away_scorer.map(|_| 1).unwrap_or(0);
        if current.score.total() + partial_goals > config.max_total_goals {
            pruned += merged_prob;
            continue;
        }

        let mut merged = current.clone();
        if let Some(scorer) = partial.home_scorer {
            merged.scorers.entry(scorer.clone()).and_modify(|count| *count += 1).or_insert(1);
            merged.score.home += 1;
        }
        if let Some(scorer) = partial.away_scorer {
            merged.scorers.entry(scorer.clone()).and_modify(|count| *count += 1).or_insert(1);
            merged.score.away += 1;
        }
        next_prospects
            .entry(merged)
            .and_modify(|prob| *prob += merged_prob)
            .or_insert(merged_prob);
    }
    pruned
}

#[cfg(test)]
mod tests;