use std::collections::BTreeMap;
use std::time::Instant;

use brumby::capture::Capture;
use brumby::linear::matrix::Matrix;
use brumby::opt::{
    hypergrid_search, univariate_descent, HypergridSearchConfig, HypergridSearchOutcome,
    UnivariateDescentConfig, UnivariateDescentOutcome,
};
use brumby::probs::SliceExt;
use brumby::{factorial, poisson};

use crate::domain::Player::Named;
use crate::domain::{Offer, OfferType, OutcomeType, Player, Side};
use crate::interval::query::isolate;
use crate::interval::{explore, Expansions, IntervalConfig, PruneThresholds, BivariateProbs, PlayerProbs, TeamProbs};
use crate::scoregrid;

// const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
// const SINGLE_PRICE_BOUNDS: PriceBounds = 1.01..=301.0;
// const FIRST_GOALSCORER_BOOKSUM: f64 = 1.0;
const INTERVALS: usize = 18;
const MAX_TOTAL_GOALS_HALF: u16 = 4;
const MAX_TOTAL_GOALS_FULL: u16 = 8;
const GOALSCORER_MIN_PROB: f64 = 0.0;
const ERROR_TYPE: ErrorType = ErrorType::SquaredRelative;

pub enum ErrorType {
    SquaredRelative,
    SquaredAbsolute,
}
impl ErrorType {
    pub fn calculate(&self, expected: f64, sample: f64) -> f64 {
        match self {
            ErrorType::SquaredRelative => ((expected - sample) / sample).powi(2),
            ErrorType::SquaredAbsolute => (expected - sample).powi(2),
        }
    }

    pub fn reverse(&self, error: f64) -> f64 {
        error.sqrt()
    }
}

pub fn fit_scoregrid_half(offers: &[&Offer]) -> HypergridSearchOutcome {
    let init_estimates = {
        let start = Instant::now();
        let search_outcome = fit_bivariate_poisson_scoregrid(offers, MAX_TOTAL_GOALS_HALF);
        let elapsed = start.elapsed();
        println!("biv-poisson: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_lambdas(&search_outcome.optimal_values));
        search_outcome
            .optimal_values
            .iter()
            .map(|optimal_value| {
                1.0 - poisson::univariate(
                    0,
                    optimal_value / INTERVALS as f64 * 2.0,
                    &factorial::Calculator,
                )
            })
            .collect::<Vec<_>>()
    };
    println!("initial estimates: {init_estimates:?}");

    let start = Instant::now();
    let search_outcome = fit_bivariate_binomial_scoregrid(
        offers,
        &init_estimates,
        (INTERVALS / 2) as u8,
        MAX_TOTAL_GOALS_HALF,
    );
    let elapsed = start.elapsed();
    println!("biv-binomial: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
    search_outcome
}

pub fn fit_scoregrid_full(offers: &[&Offer]) -> HypergridSearchOutcome {
    let init_estimates = {
        println!("*** F/T: fitting bivariate poisson scoregrid ***");
        let start = Instant::now();
        let search_outcome = fit_bivariate_poisson_scoregrid(offers, MAX_TOTAL_GOALS_FULL);
        let elapsed = start.elapsed();
        println!(
            "F/T: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}",
            expectation_from_lambdas(&search_outcome.optimal_values)
        );
        search_outcome
            .optimal_values
            .iter()
            .map(|optimal_value| {
                1.0 - poisson::univariate(
                    0,
                    optimal_value / INTERVALS as f64,
                    &factorial::Calculator,
                )
            })
            .collect::<Vec<_>>()
    };
    println!("F/T: initial estimates: {init_estimates:?}");

    println!("*** F/T: fitting bivariate binomial scoregrid ***");
    let start = Instant::now();
    let search_outcome = fit_bivariate_binomial_scoregrid(
        offers,
        &init_estimates,
        INTERVALS as u8,
        MAX_TOTAL_GOALS_FULL,
    );
    // let search_outcome = fit_scoregrid(&[&correct_score]);
    let elapsed = start.elapsed();
    println!("F/T: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
    search_outcome
}

pub fn fit_first_goalscorer_all(
    h1_probs: &BivariateProbs,
    h2_probs: &BivariateProbs,
    first_gs: &Offer,
    draw_prob: f64,
) -> BTreeMap<Player, f64> {
    let home_rate = (h1_probs.home + h2_probs.home) / 2.0;
    let away_rate = (h1_probs.away + h2_probs.away) / 2.0;
    let common_rate = (h1_probs.common + h2_probs.common) / 2.0;
    let rate_sum = home_rate + away_rate + common_rate;
    let home_ratio = (home_rate + common_rate / 2.0) / rate_sum * (1.0 - draw_prob);
    let away_ratio = (away_rate + common_rate / 2.0) / rate_sum * (1.0 - draw_prob);
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let mut fitted_goalscorer_probs = BTreeMap::new();
    let start = Instant::now();
    for (index, outcome) in first_gs.outcomes.items().iter().enumerate() {
        match outcome {
            OutcomeType::Player(player) => {
                let side_ratio = match player {
                    Named(side, _) => match side {
                        Side::Home => home_ratio,
                        Side::Away => away_ratio,
                    },
                    Player::Other => unreachable!(),
                };
                let init_estimate = first_gs.market.probs[index] / side_ratio;
                let player_search_outcome = fit_first_goalscorer_one(
                    h1_probs,
                    h2_probs,
                    player,
                    init_estimate,
                    first_gs.market.probs[index],
                );
                // println!("goal for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", first_gs.market.probs[index]);
                fitted_goalscorer_probs.insert(player.clone(), player_search_outcome.optimal_value);
            }
            OutcomeType::None => {
            }
            _ => unreachable!(),
        }
    }
    let elapsed = start.elapsed();
    println!("first goalscorer fitting took {elapsed:?}");
    fitted_goalscorer_probs
}

fn fit_first_goalscorer_one(
    h1_goals: &BivariateProbs,
    h2_goals: &BivariateProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
) -> UnivariateDescentOutcome {
    let mut config = IntervalConfig {
        intervals: INTERVALS as u8,
        team_probs: TeamProbs {
            h1_goals: h1_goals.clone(),
            h2_goals: h2_goals.clone(),
        },
        player_probs: vec![(player.clone(), PlayerProbs { goal: Some(0.0), assist: None })],
        prune_thresholds: PruneThresholds {
            max_total_goals: MAX_TOTAL_GOALS_FULL,
            min_prob: GOALSCORER_MIN_PROB,
        },
        expansions: Expansions {
            ht_score: false,
            ft_score: false,
            player_goal_stats: false,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: true,
        },
    };
    let outcome_type = OutcomeType::Player(player.clone());
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            config.player_probs[0].1.goal = Some(value);
            let exploration = explore(&config, 0..INTERVALS as u8);
            let isolated_prob = isolate(
                &OfferType::FirstGoalscorer,
                &outcome_type,
                &exploration.prospects,
                &exploration.player_lookup,
            );
            ERROR_TYPE.calculate(expected_prob, isolated_prob)
        },
    )
}

pub fn fit_anytime_assist_all(
    h1_probs: &BivariateProbs,
    h2_probs: &BivariateProbs,
    anytime_assist: &Offer,
    draw_prob: f64,
    booksum: f64,
) -> BTreeMap<Player, f64> {
    let home_rate = (h1_probs.home + h2_probs.home) / 2.0;
    let away_rate = (h1_probs.away + h2_probs.away) / 2.0;
    let common_rate = (h1_probs.common + h2_probs.common) / 2.0;
    let rate_sum = home_rate + away_rate + common_rate;
    let home_ratio = (home_rate + common_rate / 2.0) / rate_sum * (1.0 - draw_prob);
    let away_ratio = (away_rate + common_rate / 2.0) / rate_sum * (1.0 - draw_prob);
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let mut fitted_assist_probs = BTreeMap::new();
    let start = Instant::now();
    for (index, outcome) in anytime_assist.outcomes.items().iter().enumerate() {
        match outcome {
            OutcomeType::Player(player) => {
                let side_ratio = match player {
                    Named(side, _) => match side {
                        Side::Home => home_ratio,
                        Side::Away => away_ratio,
                    },
                    Player::Other => unreachable!(),
                };
                let init_estimate = anytime_assist.market.probs[index] / booksum / side_ratio;
                let player_search_outcome = fit_anytime_assist_one(
                    h1_probs,
                    h2_probs,
                    player,
                    init_estimate,
                    anytime_assist.market.probs[index],
                );
                // println!("assist for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", anytime_assist.market.probs[index]);
                fitted_assist_probs.insert(player.clone(), player_search_outcome.optimal_value);
            }
            OutcomeType::None => {}
            _ => unreachable!(),
        }
    }
    let elapsed = start.elapsed();
    println!("anytime assist fitting took {elapsed:?}");
    fitted_assist_probs
}

fn fit_anytime_assist_one(
    h1_goals: &BivariateProbs,
    h2_goals: &BivariateProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
) -> UnivariateDescentOutcome {
    let mut config = IntervalConfig {
        intervals: INTERVALS as u8,
        team_probs: TeamProbs {
            h1_goals: h1_goals.clone(),
            h2_goals: h2_goals.clone(),
        },
        player_probs: vec![(player.clone(), PlayerProbs { goal: None, assist: Some(0.0) })],
        prune_thresholds: PruneThresholds {
            max_total_goals: MAX_TOTAL_GOALS_FULL,
            min_prob: GOALSCORER_MIN_PROB,
        },
        expansions: Expansions {
            ht_score: false,
            ft_score: false,
            player_goal_stats: false,
            player_split_goal_stats: false,
            max_player_assists: 1,
            first_goalscorer: false,
        },
    };
    let outcome_type = OutcomeType::Player(player.clone());
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            config.player_probs[0].1.assist = Some(value);
            let exploration = explore(&config, 0..INTERVALS as u8);
            let isolated_prob = isolate(
                &OfferType::AnytimeAssist,
                &outcome_type,
                &exploration.prospects,
                &exploration.player_lookup,
            );
            ERROR_TYPE.calculate(expected_prob, isolated_prob)
        },
    )
}

fn expectation_from_lambdas(lambdas: &[f64]) -> f64 {
    assert_eq!(3, lambdas.len());
    lambdas[0] + lambdas[1] + 2.0 * lambdas[2]
}

fn allocate_scoregrid(max_total_goals: u16) -> Matrix<f64> {
    let dim = usize::min(max_total_goals as usize, INTERVALS) + 1;
    Matrix::allocate(dim, dim)
}

fn fit_bivariate_poisson_scoregrid(
    offers: &[&Offer],
    max_total_goals: u16,
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(max_total_goals);
    let bounds = [0.2..=3.0, 0.2..=3.0, 0.0..=0.5];
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: Capture::Borrowed(&bounds),
            resolution: 10,
        },
        |_| true,
        |values| {
            bivariate_poisson_scoregrid(values[0], values[1], values[2], &mut scoregrid);
            scoregrid_error(offers, &scoregrid)
        },
    )
}

fn fit_bivariate_binomial_scoregrid(
    offers: &[&Offer],
    init_estimates: &[f64],
    intervals: u8,
    max_total_goals: u16,
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(max_total_goals);
    let bounds = init_estimates
        .iter()
        .map(|&estimate| (estimate * 0.67)..=(estimate * 1.5))
        .collect::<Vec<_>>();
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: Capture::Borrowed(&bounds),
            resolution: 10,
        },
        |values| values.sum() <= 1.0,
        |values| {
            bivariate_binomial_scoregrid(
                intervals,
                values[0],
                values[1],
                values[2],
                &mut scoregrid,
            );
            scoregrid_error(offers, &scoregrid)
        },
    )
}

/// Bivariate Poisson.
fn bivariate_poisson_scoregrid(
    home_rate: f64,
    away_rate: f64,
    common: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_bivariate_poisson(home_rate, away_rate, common, scoregrid);
    // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
}

/// Bivariate binomial.
fn bivariate_binomial_scoregrid(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    interval_common_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_bivariate_binomial(
        intervals,
        interval_home_prob,
        interval_away_prob,
        interval_common_prob,
        scoregrid,
    );
}

fn scoregrid_error(offers: &[&Offer], scoregrid: &Matrix<f64>) -> f64 {
    let mut residual = 0.0;
    for offer in offers {
        for (index, outcome) in offer.outcomes.items().iter().enumerate() {
            let fitted_prob = outcome.gather(scoregrid);
            let sample_prob = offer.market.probs[index];
            residual += ERROR_TYPE.calculate(sample_prob, fitted_prob);
        }
    }
    residual
}

pub fn compute_error(sample_prices: &[f64], fitted_prices: &[f64], error_type: &ErrorType) -> f64 {
    let mut error_sum = 0.0;
    let mut counted = 0;
    for (index, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[index];
        if fitted_price.is_finite() {
            counted += 1;
            let (sample_prob, fitted_prob) = (1.0 / sample_price, 1.0 / fitted_price);
            error_sum += error_type.calculate(sample_prob, fitted_prob);
        }
    }
    let mean_error = error_sum / counted as f64;
    error_type.reverse(mean_error)
}
