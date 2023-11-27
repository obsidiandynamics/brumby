use ordinalizer::Ordinal;
use crate::comb::{count_permutations, pick};
use crate::linear::matrix::Matrix;

#[derive(Debug, Ordinal)]
pub enum GoalEvent {
    Neither,
    Home,
    Away,
    Both
}
impl GoalEvent {
    pub fn is_home(&self) -> bool {
        matches!(self, GoalEvent::Home | GoalEvent::Both)
    }

    pub fn is_away(&self) -> bool {
        matches!(self, GoalEvent::Away | GoalEvent::Both)
    }
}

impl From<usize> for GoalEvent {
    #[inline]
    fn from(value: usize) -> Self {
        match value {
            0 => GoalEvent::Neither,
            1 => GoalEvent::Home,
            2 => GoalEvent::Away,
            3 => GoalEvent::Both,
            _ => unreachable!()
        }
    }
}

#[derive(Debug)]
pub struct ScoreOutcome {
    pub home_goals: usize,
    pub away_goals: usize,
    pub probability: f64
}

#[derive(Debug)]
pub struct ScoreOutcomeSpace {
    pub interval_home_prob: f64,
    pub interval_away_prob: f64
}

pub struct Iter<'a> {
    fixtures: &'a mut IterFixtures,
    permutation: u64,
    neither_prob: f64,
    home_only_prob: f64,
    away_only_prob: f64,
    both_prob: f64,
}
impl<'a> Iter<'a> {
    pub fn new(space: &'a ScoreOutcomeSpace, fixtures: &'a mut IterFixtures) -> Self {
        let both_prob = space.interval_home_prob * space.interval_away_prob;
        let home_only_prob = space.interval_home_prob * (1.0 - space.interval_away_prob);
        let away_only_prob = space.interval_away_prob * (1.0 - space.interval_home_prob);
        let neither_prob = 1.0 - home_only_prob - away_only_prob - both_prob;
        Self {
            fixtures,
            permutation: 0,
            neither_prob,
            home_only_prob,
            away_only_prob,
            both_prob
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ScoreOutcome;

    fn next(&mut self) -> Option<Self::Item> {
        if self.permutation < self.fixtures.permutations {
            pick(&self.fixtures.cardinalities, self.permutation, &mut self.fixtures.ordinals);
            let mut probability = 1.0;
            let (mut home_goals, mut away_goals) = (0, 0);
            for &ordinal in self.fixtures.ordinals.iter() {
                let goal_event = GoalEvent::from(ordinal);
                match goal_event {
                    GoalEvent::Neither => {
                        probability *= self.neither_prob;
                    }
                    GoalEvent::Home => {
                        probability *= self.home_only_prob;
                        home_goals += 1;
                    }
                    GoalEvent::Away => {
                        probability *= self.away_only_prob;
                        away_goals += 1;
                    }
                    GoalEvent::Both => {
                        probability *= self.both_prob;
                        home_goals += 1;
                        away_goals += 1;
                    }
                }
            }
            self.permutation += 1;
            Some(ScoreOutcome {
                home_goals,
                away_goals,
                probability,
            })
        } else {
            None
        }
    }
}

pub struct IterFixtures {
    cardinalities: Vec<usize>,
    ordinals: Vec<usize>,
    permutations: u64
}
impl IterFixtures {
    pub fn new(intervals: usize) -> Self {
        let cardinalities = vec![4; intervals];
        let permutations = count_permutations(&cardinalities);
        let ordinals = cardinalities.clone();
        Self {
            cardinalities,
            ordinals,
            permutations,
        }
    }
}

pub fn populate_matrix(iter: Iter, scoregrid: &mut Matrix<f64>) {
    debug_assert_eq!(iter.fixtures.cardinalities.len() + 1, scoregrid.rows());
    debug_assert_eq!(iter.fixtures.cardinalities.len() + 1, scoregrid.cols());

    for outcome in iter {
        scoregrid[(outcome.home_goals, outcome.away_goals)] += outcome.probability;
    }
}

// pub fn enumerate_score_outcomes(intervals: usize, interval_home_prob: f64, interval_away_prob: f64) -> Vec<ScoreOutcome> {
//     let cardinalities = vec![4; intervals];
//     let permutations = count_permutations(&cardinalities);
//     let mut ordinals = cardinalities.clone();
//     let mut outcomes = Vec::with_capacity(permutations as usize);
//
//     let both_prob = interval_home_prob * interval_away_prob;
//     let neither_prob = 1.0 - interval_home_prob - interval_away_prob - both_prob;
//     for permutation in 0..permutations {
//         pick(&cardinalities, permutation, &mut ordinals);
//         let mut probability = 1.0;
//         let (mut home_goals, mut away_goals) = (0, 0);
//         for &ordinal in ordinals.iter() {
//             let goal_event = GoalEvent::from(ordinal);
//             match goal_event {
//                 GoalEvent::Neither => {
//                     probability *= neither_prob;
//                 }
//                 GoalEvent::Home => {
//                     probability *= interval_home_prob;
//                     home_goals += 1;
//                 }
//                 GoalEvent::Away => {
//                     probability *= interval_away_prob;
//                     away_goals += 1;
//                 }
//                 GoalEvent::Both => {
//                     probability *= both_prob;
//                     home_goals += 1;
//                     away_goals += 1;
//                 }
//             }
//         }
//         outcomes.push(ScoreOutcome {
//             home_goals,
//             away_goals,
//             probability,
//         })
//     }
//
//     outcomes
// }

#[cfg(test)]
mod tests;