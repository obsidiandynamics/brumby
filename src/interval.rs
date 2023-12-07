use std::ops::Range;
use rustc_hash::FxHashMap;

use crate::domain::{MarketType, OutcomeType, Player, Score, Side};

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

#[derive(Debug, Clone)]
pub struct ScoringProbs {
    pub home_prob: f64,
    pub away_prob: f64,
    pub common_prob: f64,
}

impl<'a> From<&'a [f64]> for ScoringProbs {
    fn from(params: &'a [f64]) -> Self {
        assert_eq!(3, params.len());
        Self {
            home_prob: params[0],
            away_prob: params[1],
            common_prob: params[2],
        }
    }
}

#[derive(Debug)]
pub struct IntervalConfig {
    pub intervals: u8,
    pub h1_probs: ScoringProbs,
    pub h2_probs: ScoringProbs,
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

pub fn explore(config: &IntervalConfig, explore_intervals: Range<u8>) -> Exploration {
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
    let mut pruned = 0.0;

    for interval in explore_intervals {
        let params = if interval < config.intervals / 2 {
            &config.h1_probs
        } else {
            &config.h2_probs
        };

        let neither_prob = 1.0 - params.home_prob - params.away_prob - params.common_prob;

        let mut next_prospects = init_prospects((current_prospects.len() as f64 * 1.1) as usize);

        for (current_prospect, current_prob) in current_prospects {
            // neither team scores
            let partial = PartialProspect {
                home_scorer: None,
                away_scorer: None,
                first_scoring_side: None,
                prob: neither_prob,
            };
            merge(&current_prospect, current_prob, partial, &mut next_prospects);

            // at least one more goal allowed before pruning
            if current_prospect.score.total() < config.max_total_goals {
                // only the home team scores
                for (player_index, player_prob) in &home_scorers {
                    let partial = PartialProspect {
                        home_scorer: Some(*player_index),
                        away_scorer: None,
                        first_scoring_side: Some(&Side::Home),
                        prob: params.home_prob * player_prob,
                    };
                    merge(&current_prospect, current_prob, partial, &mut next_prospects);
                }

                // only the away team scores
                for (player_index, player_prob) in &away_scorers {
                    let partial = PartialProspect {
                        home_scorer: None,
                        away_scorer: Some(*player_index),
                        first_scoring_side: Some(&Side::Away),
                        prob: params.away_prob * player_prob,
                    };
                    merge(&current_prospect, current_prob, partial, &mut next_prospects);
                }
            } else {
                pruned += current_prob * (params.home_prob + params.away_prob);
            }

            // at least two more goals allowed before pruning
            if current_prospect.score.total() + 1 < config.max_total_goals {
                // both teams score
                for (home_player_index, home_player_prob) in &home_scorers {
                    for (away_player_index, away_player_prob) in &away_scorers {
                        for first_scoring_side in [&Side::Home, &Side::Away] {
                            let partial = PartialProspect {
                                home_scorer: Some(*home_player_index),
                                away_scorer: Some(*away_player_index),
                                first_scoring_side: Some(first_scoring_side),
                                prob: params.common_prob * home_player_prob * away_player_prob * 0.5,
                            };
                            merge(&current_prospect, current_prob, partial, &mut next_prospects);
                        }
                    }
                }
            } else {
                pruned += current_prob * params.common_prob;
            }
        }

        current_prospects = next_prospects;
    }

    Exploration {
        player_lookup,
        prospects: current_prospects,
        pruned,
    }
}

#[inline]
fn merge(
    current_prospect: &Prospect,
    current_prob: f64,
    partial: PartialProspect,
    next_prospects: &mut Prospects,
) {
        let merged_prob = current_prob * partial.prob;
        let mut merged = current_prospect.clone();
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

#[must_use]
pub fn isolate(
    market_type: &MarketType,
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &[Player],
) -> f64 {
    match market_type {
        MarketType::HeadToHead(_) => unimplemented!(),
        MarketType::TotalGoalsOverUnder(_, _) => unimplemented!(),
        MarketType::CorrectScore(_) => unimplemented!(),
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
