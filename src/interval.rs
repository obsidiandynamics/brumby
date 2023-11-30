use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;
use crate::scoregrid::{GoalEvent, Score};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scenario {
    pub score: Score,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            score: Score { home: 0, away: 0},
        }
    }
}

pub type Scenarios = FxHashMap<Scenario, f64>;

pub fn init_scenarios(capacity: usize) -> Scenarios {
    Scenarios::with_capacity_and_hasher(capacity, Default::default())
}

#[derive(Debug)]
pub struct IntervalConfig {
    pub intervals: u8,
    pub home_prob: f64,
    pub away_prob: f64,
    pub common_prob: f64,
    pub max_total_goals: u16
}

// fn explore_interval(config: &IntervalConfig) -> Scenarios {
//     let mut scenarios = init_scenarios(4);
//     scenarios.insert(
//         Scenario {
//             score: Score { home: 0, away: 0 },
//         },
//         1.0 - config.home_prob - config.away_prob - config.common_prob,
//     );
//     scenarios.insert(
//         Scenario {
//             score: Score { home: 1, away: 0 },
//         },
//         config.home_prob,
//     );
//     scenarios.insert(
//         Scenario {
//             score: Score { home: 0, away: 1 },
//         },
//         config.away_prob,
//     );
//     scenarios.insert(
//         Scenario {
//             score: Score { home: 1, away: 1 },
//         },
//         config.common_prob,
//     );
//     scenarios
// }
//
// fn fold_intervals(a: &Scenarios, b: &Scenarios) -> Scenarios {
//     let mut mutations = init_scenarios(a.len() * b.len());
//     for (a_scenario, a_prob) in a {
//         for (b_scenario, b_prob) in b {
//             let mutation = Scenario {
//                 score: Score {
//                     home: a_scenario.score.home + b_scenario.score.home,
//                     away: a_scenario.score.away + b_scenario.score.away,
//                 },
//             };
//             let mutation_prob = a_prob * b_prob;
//             mutations
//                 .entry(mutation)
//                 .and_modify(|prob| *prob += mutation_prob)
//                 .or_insert(mutation_prob);
//         }
//     }
//     mutations
// }
//
// pub fn explore_all(config: &IntervalConfig) -> Scenarios {
//     let mut scenarios = explore_interval(config);
//     for _ in 1..config.intervals {
//         let next_scenarios = explore_interval(config);
//         scenarios = fold_intervals(&scenarios, &next_scenarios);
//     }
//     scenarios
// }

#[derive(Debug)]
pub struct Exploration {
    pub scenarios: Scenarios,
    pub pruned: f64
}

pub fn explore_all(config: &IntervalConfig) -> Exploration {
    let mut current_scenarios = init_scenarios(1);
    current_scenarios.insert(Scenario::default(), 1.0);
    let neither_prob = 1.0 - config.home_prob - config.away_prob - config.common_prob;
    let mut pruned = 0.0;

    for _ in 0..config.intervals {
        let mut next_scenarios = init_scenarios(current_scenarios.len() * 4);
        for goal_event in GoalEvent::iter() {
            let (next, next_prob) = match goal_event {
                GoalEvent::Neither => {
                    (Scenario {
                        score: Score { home: 0, away: 0 },
                    }, neither_prob)
                }
                GoalEvent::Home => {
                    (Scenario {
                        score: Score { home: 1, away: 0 },
                    }, config.home_prob)
                }
                GoalEvent::Away => {
                    (Scenario {
                        score: Score { home: 0, away: 1 },
                    }, config.away_prob)
                }
                GoalEvent::Both => {
                    (Scenario {
                        score: Score { home: 1, away: 1 },
                    }, config.common_prob)
                }
            };

            for (current, current_prob) in &current_scenarios {
                let merged_prob = current_prob * next_prob;
                if current.score.total() + next.score.total() > config.max_total_goals {
                    pruned += merged_prob;
                    continue;
                }
                let merged = Scenario {
                    score: Score {
                        home: current.score.home + next.score.home,
                        away: current.score.away + next.score.away,
                    },
                };
                next_scenarios
                    .entry(merged)
                    .and_modify(|prob| *prob += merged_prob)
                    .or_insert(merged_prob);
            }
        }
        current_scenarios = next_scenarios;
    }

    Exploration {
        scenarios: current_scenarios,
        pruned
    }
}

#[cfg(test)]
mod tests;