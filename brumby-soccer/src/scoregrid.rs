use std::ops::Range;
use ordinalizer::Ordinal;
use strum_macros::{EnumCount, EnumIter};

use brumby::multinomial::binomial;

use brumby::comb::{count_permutations, pick};
use crate::domain::{OutcomeType, Score, Side};
use crate::interval::{Expansions, explore, IntervalConfig, PruneThresholds, ScoringProbs};
use brumby::linear::matrix::Matrix;
use brumby::multinomial::bivariate_binomial;
use brumby::probs::SliceExt;
use brumby::{factorial, poisson};

#[derive(Debug, Ordinal, EnumCount, EnumIter)]
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

#[derive(Debug)]
pub struct ProbableScoreOutcome {
    pub score: Score,
    pub probability: f64,
}

#[derive(Debug)]
pub struct ScoreOutcomeSpace {
    pub interval_home_prob: f64,
    pub interval_away_prob: f64,
    pub interval_common_prob: f64,
}

pub struct Iter<'a> {
    space: &'a ScoreOutcomeSpace,
    fixtures: &'a mut IterFixtures,
    permutation: u64,
    interval_neither_prob: f64,
}
impl<'a> Iter<'a> {
    pub fn new(space: &'a ScoreOutcomeSpace, fixtures: &'a mut IterFixtures) -> Self {
        let interval_neither_prob =
            1.0 - space.interval_home_prob - space.interval_away_prob - space.interval_common_prob;
        Self {
            space,
            fixtures,
            permutation: 0,
            interval_neither_prob,
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
                        probability *= self.interval_neither_prob;
                    }
                    GoalEvent::Home => {
                        probability *= self.space.interval_home_prob;
                        home_goals += 1;
                    }
                    GoalEvent::Away => {
                        probability *= self.space.interval_away_prob;
                        away_goals += 1;
                    }
                    GoalEvent::Both => {
                        probability *= self.space.interval_common_prob;
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

// pub fn from_interval(
//     interval_home_prob: f64,
//     interval_away_prob: f64,
//     interval_common_prob: f64,
//     scoregrid: &mut Matrix<f64>,
// ) {
//     assert_eq!(scoregrid.rows(), scoregrid.cols());
//     let intervals = scoregrid.rows() - 1;
//     let space = ScoreOutcomeSpace {
//         interval_home_prob,
//         interval_away_prob,
//         interval_common_prob,
//     };
//     let mut fixtures = IterFixtures::new(intervals);
//     let iter = Iter::new(&space, &mut fixtures);
//     from_iterator(iter, scoregrid);
// }
pub fn from_interval(
    intervals: u8,
    explore_intervals: Range<u8>,
    max_total_goals: u16,
    h1_params: ScoringProbs,
    h2_params: ScoringProbs,
    scoregrid: &mut Matrix<f64>,
) {
    assert_eq!(scoregrid.rows(), scoregrid.cols());
    let exploration = explore(
        &IntervalConfig {
            intervals,
            h1_probs: h1_params,
            h2_probs: h2_params,
            players: vec![],
            prune_thresholds: PruneThresholds {
                max_total_goals,
                min_prob: 0.0,
            },
            expansions: Expansions {
                ft_score: true,
                player_stats: false,
                player_split_stats: false,
                first_goalscorer: false,
            }
        },
        explore_intervals,
    );
    for (scenario, prob) in exploration.prospects {
        scoregrid[(scenario.score.home as usize, scenario.score.away as usize)] += prob;
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

pub fn from_binomial(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    let factorial = factorial::Calculator;
    for home_goals in 0..=u8::min(intervals, (scoregrid.rows() - 1) as u8) {
        for away_goals in 0..=u8::min(intervals, (scoregrid.cols() - 1) as u8) {
            let home_prob = binomial(intervals, home_goals, interval_home_prob, &factorial);
            let away_prob = binomial(intervals, away_goals, interval_away_prob, &factorial);
            scoregrid[(home_goals as usize, away_goals as usize)] = home_prob * away_prob;
        }
    }
}

pub fn from_bivariate_binomial(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    interval_common_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    assert_eq!(scoregrid.rows(), scoregrid.cols());
    let factorial = factorial::Calculator;
    for home_goals in 0..=u8::min(intervals, (scoregrid.rows() - 1) as u8) {
        for away_goals in 0..=u8::min(intervals, (scoregrid.cols() - 1) as u8) {
            scoregrid[(home_goals as usize, away_goals as usize)] = bivariate_binomial(
                intervals,
                home_goals,
                away_goals,
                interval_home_prob,
                interval_away_prob,
                interval_common_prob,
                &factorial,
            );
        }
    }
}

pub fn from_correct_score(outcomes: &[OutcomeType], probs: &[f64], scoregrid: &mut Matrix<f64>) {
    for (index, outcome) in outcomes.iter().enumerate() {
        match outcome {
            OutcomeType::Score(score) => {
                if (score.home as usize) < scoregrid.rows()
                    && (score.away as usize) < scoregrid.cols()
                {
                    scoregrid[(score.home as usize, score.away as usize)] += probs[index];
                }
            }
            _ => panic!("unexpected {outcome:?}"),
        }
    }
}

pub fn home_away_expectations(scoregrid: &Matrix<f64>) -> (f64, f64) {
    let (mut home_expectation, mut away_expectation) = (0.0, 0.0);

    for home_goals in 0..scoregrid.rows() {
        for away_goals in 0..scoregrid.cols() {
            let prob = scoregrid[(home_goals, away_goals)];
            home_expectation += home_goals as f64 * prob;
            away_expectation += away_goals as f64 * prob;
        }
    }

    (home_expectation, away_expectation)
}

pub fn subtract(future: &Matrix<f64>, past: &Matrix<f64>) -> Matrix<f64> {
    assert_eq!(future.rows(), past.rows());
    assert_eq!(future.cols(), past.cols());
    let (home_goals, away_goals) = (future.rows(), future.cols());
    let mut diff = Matrix::allocate(home_goals, away_goals);

    for past_home_goals in 0..past.rows() {
        for past_away_goals in 0..past.cols() {
            let past_prob = past[(past_home_goals, past_away_goals)];
            // println!("{past_home_goals}:{past_away_goals} => past_prob={past_prob}");
            let mut remaining_prob = 0.0;
            for future_home_goals in past_home_goals..future.rows() {
                for future_away_goals in past_away_goals..future.cols() {
                    let future_prob = future[(future_home_goals, future_away_goals)];
                    remaining_prob += future_prob;
                }
            }
            // println!("  {past_home_goals}:{past_away_goals} remaining_prob={remaining_prob}");
            if remaining_prob > 0.0 {
                for future_home_goals in past_home_goals..future.rows() {
                    for future_away_goals in past_away_goals..future.cols() {
                        let future_prob = future[(future_home_goals, future_away_goals)];
                        let past_to_future_prob = past_prob * future_prob / remaining_prob;
                        // println!("  {future_home_goals}:{future_away_goals} future_prob={future_prob}, past_to_future_prob={past_to_future_prob}");
                        diff[(future_home_goals - past_home_goals, future_away_goals - past_away_goals)] += past_to_future_prob;
                    }
                }
            }
        }
    }

    diff
}

pub fn inflate_zero(additive: f64, scoregrid: &mut Matrix<f64>) {
    scoregrid[(0, 0)] += additive;
    scoregrid.flatten_mut().normalise(1.0);
}

impl OutcomeType {
    pub fn gather(&self, scoregrid: &Matrix<f64>) -> f64 {
        match self {
            OutcomeType::Win(side) => Self::gather_win(side, scoregrid),
            OutcomeType::Draw => Self::gather_draw(scoregrid),
            OutcomeType::Under(goals) => Self::gather_goals_under(*goals, scoregrid),
            OutcomeType::Over(goals) => Self::gather_goals_over(*goals, scoregrid),
            OutcomeType::Score(score) => Self::gather_correct_score(score, scoregrid),
            _ => unimplemented!(),
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
        if (score.home as usize) < scoregrid.rows() && (score.away as usize) < scoregrid.cols() {
            scoregrid[(score.home as usize, score.away as usize)]
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests;
