use rustc_hash::FxHashMap;
use crate::scoregrid::Score;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scenario {
    pub score: Score,
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
}

fn explore_interval(config: &IntervalConfig) -> Scenarios {
    let mut scenarios = init_scenarios(4);
    scenarios.insert(
        Scenario {
            score: Score { home: 0, away: 0 },
        },
        1.0 - config.home_prob - config.away_prob - config.common_prob,
    );
    scenarios.insert(
        Scenario {
            score: Score { home: 1, away: 0 },
        },
        config.home_prob,
    );
    scenarios.insert(
        Scenario {
            score: Score { home: 0, away: 1 },
        },
        config.away_prob,
    );
    scenarios.insert(
        Scenario {
            score: Score { home: 1, away: 1 },
        },
        config.common_prob,
    );
    scenarios
}

fn fold_intervals(a: &Scenarios, b: &Scenarios) -> Scenarios {
    let mut mutations = init_scenarios(a.len() * b.len());
    for (a_scenario, a_prob) in a {
        for (b_scenario, b_prob) in b {
            let mutation = Scenario {
                score: Score {
                    home: a_scenario.score.home + b_scenario.score.home,
                    away: a_scenario.score.away + b_scenario.score.away,
                },
            };
            let mutation_prob = a_prob * b_prob;
            mutations
                .entry(mutation)
                .and_modify(|prob| *prob += mutation_prob)
                .or_insert(mutation_prob);
        }
    }
    mutations
}

pub fn explore_all(config: &IntervalConfig) -> Scenarios {
    let mut scenarios = explore_interval(config);
    for _ in 1..config.intervals {
        let next_scenarios = explore_interval(config);
        scenarios = fold_intervals(&scenarios, &next_scenarios);
    }
    scenarios
}

#[cfg(test)]
mod tests;