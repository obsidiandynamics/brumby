use ordinalizer::Ordinal;

use crate::comb::{count_permutations, pick};
use crate::linear::matrix::Matrix;
use crate::probs::SliceExt;
use crate::{factorial, poisson};

#[derive(Debug, Ordinal)]
pub enum GoalEvent {
    Neither,
    Home,
    Away,
    Both,
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
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub home: u8,
    pub away: u8,
}
impl Score {
    pub fn new(home: u8, away: u8) -> Self {
        Self { home, away }
    }
}

#[derive(Debug)]
pub struct ProbableScoreOutcome {
    pub score: Score,
    pub probability: f64,
}

#[derive(Debug)]
pub struct ScoreOutcomeSpace {
    pub interval_home_prob: f64,
    pub interval_away_prob: f64,
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
            both_prob,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ProbableScoreOutcome;

    fn next(&mut self) -> Option<Self::Item> {
        if self.permutation < self.fixtures.permutations {
            pick(
                &self.fixtures.cardinalities,
                self.permutation,
                &mut self.fixtures.ordinals,
            );
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
            Some(ProbableScoreOutcome {
                score: Score {
                    home: home_goals,
                    away: away_goals,
                },
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
    permutations: u64,
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

pub fn from_iterator(iter: Iter, scoregrid: &mut Matrix<f64>) {
    debug_assert_eq!(iter.fixtures.cardinalities.len() + 1, scoregrid.rows());
    debug_assert_eq!(iter.fixtures.cardinalities.len() + 1, scoregrid.cols());

    for outcome in iter {
        scoregrid[(outcome.score.home as usize, outcome.score.away as usize)] +=
            outcome.probability;
    }
}

pub fn from_univariate_poisson(home_rate: f64, away_rate: f64, scoregrid: &mut Matrix<f64>) {
    let factorial = factorial::Calculator;
    for home_goals in 0..scoregrid.rows() {
        for away_goals in 0..scoregrid.cols() {
            let home_prob = poisson::univariate(home_goals as u8, home_rate, &factorial);
            let away_prob = poisson::univariate(away_goals as u8, away_rate, &factorial);
            scoregrid[(home_goals, away_goals)] = home_prob * away_prob;
        }
    }
}

pub fn from_bivariate_poisson(
    home_rate: f64,
    away_rate: f64,
    common_rate: f64,
    scoregrid: &mut Matrix<f64>,
) {
    let factorial = factorial::Calculator;
    for home_goals in 0..scoregrid.rows() {
        for away_goals in 0..scoregrid.cols() {
            scoregrid[(home_goals, away_goals)] = poisson::bivariate(
                home_goals as u8,
                away_goals as u8,
                home_rate,
                away_rate,
                common_rate,
                &factorial,
            );
        }
    }
}

// pub fn from_wierd(home_rate: f64, away_rate: f64, common: f64, scoregrid: &mut Matrix<f64>) {
//     let lambda_1 = home_rate - common;
//     let lambda_2 = away_rate - common;
//     let factorial = factorial::Calculator;
//     for home_goals in 0..scoregrid.rows() {
//         for away_goals in 0..scoregrid.cols() {
//             let home_prob = poisson::univariate(home_goals as u8, lambda_1, &factorial);
//             let away_prob = poisson::univariate(away_goals as u8, lambda_2, &factorial);
//             scoregrid[(home_goals, away_goals)] = home_prob * away_prob;
//         }
//     }
//     for both_goals in 1..scoregrid.rows() {
//         let both_probs =  poisson::univariate(both_goals as u8, common, &factorial);
//         let mut eligibles = 0;
//         for home_goals in 0..scoregrid.rows() {
//             for away_goals in 0..scoregrid.cols() {
//                 if home_goals >= both_goals && away_goals >= both_goals {
//                     eligibles += 1;
//                 }
//             }
//         }
//         for home_goals in 0..scoregrid.rows() {
//             for away_goals in 0..scoregrid.cols() {
//                 if home_goals >= both_goals && away_goals >= both_goals {
//                     scoregrid[(home_goals, away_goals)] += both_probs / eligibles as f64;
//                 }
//             }
//         }
//     }
//     scoregrid.flatten_mut().normalise(1.0);
// }

pub fn from_binomial(
    interval_home_prob: f64,
    interval_away_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    let factorial = factorial::Calculator;
    let home_intervals = scoregrid.rows() as u8 - 1;
    let away_intervals = scoregrid.cols() as u8 - 1;
    for home_goals in 0..=home_intervals {
        for away_goals in 0..=away_intervals {
            let home_prob =
                factorial::binomial(home_intervals, home_goals, interval_home_prob, &factorial);
            let away_prob =
                factorial::binomial(away_intervals, away_goals, interval_away_prob, &factorial);
            scoregrid[(home_goals as usize, away_goals as usize)] = home_prob * away_prob;
        }
    }
}

pub fn inflate_zero(additive: f64, scoregrid: &mut Matrix<f64>) {
    scoregrid[(0, 0)] += additive;
    scoregrid.flatten_mut().normalise(1.0);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Win(Side),
    Draw,
    GoalsUnder(u8),
    GoalsOver(u8),
    CorrectScore(Score),
}
impl Outcome {
    pub fn gather(&self, scoregrid: &Matrix<f64>) -> f64 {
        match self {
            Outcome::Win(side) => Self::gather_win(side, scoregrid),
            Outcome::Draw => Self::gather_draw(scoregrid),
            Outcome::GoalsUnder(goals) => Self::gather_goals_under(*goals, scoregrid),
            Outcome::GoalsOver(goals) => Self::gather_goals_over(*goals, scoregrid),
            Outcome::CorrectScore(score) => Self::gather_correct_score(score, scoregrid),
        }
    }

    fn gather_win(side: &Side, scoregrid: &Matrix<f64>) -> f64 {
        let mut prob = 0.0;
        match side {
            Side::Home => {
                for row in 1..scoregrid.rows() {
                    for col in 0..row {
                        prob += scoregrid[(row, col)];
                    }
                }
            }
            Side::Away => {
                for col in 1..scoregrid.cols() {
                    for row in 0..col {
                        prob += scoregrid[(row, col)];
                    }
                }
            }
        }
        prob
    }

    fn gather_draw(scoregrid: &Matrix<f64>) -> f64 {
        let mut prob = 0.0;
        for index in 0..scoregrid.rows() {
            prob += scoregrid[(index, index)];
        }
        prob
    }

    fn gather_goals_over(goals: u8, scoregrid: &Matrix<f64>) -> f64 {
        let goals = goals as usize;
        let mut prob = 0.0;
        for row in 0..scoregrid.rows() {
            for col in 0..scoregrid.cols() {
                if row + col > goals {
                    prob += scoregrid[(row, col)];
                }
            }
        }
        prob
    }

    fn gather_goals_under(goals: u8, scoregrid: &Matrix<f64>) -> f64 {
        let goals = goals as usize;
        let mut prob = 0.0;
        for row in 0..scoregrid.rows() {
            for col in 0..scoregrid.cols() {
                if row + col < goals {
                    prob += scoregrid[(row, col)];
                }
            }
        }
        prob
    }

    fn gather_correct_score(score: &Score, scoregrid: &Matrix<f64>) -> f64 {
        scoregrid[(score.home as usize, score.away as usize)]
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Home,
    Away,
}

#[cfg(test)]
mod tests;
