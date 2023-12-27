use std::time::Instant;

use tracing::debug;

use brumby::{factorial, poisson, sv};
use brumby::capture::Capture;
use brumby::linear::matrix::Matrix;
use brumby::opt::{
    hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome, univariate_descent,
    UnivariateDescentConfig, UnivariateDescentOutcome,
};
use brumby::probs::SliceExt;

use crate::domain::{Offer, OfferType, OutcomeType, Player, Side};
use crate::domain::Player::Named;
use crate::interval::{
    BivariateProbs, Config, explore, PlayerProbs, PruneThresholds, TeamProbs,
    UnivariateProbs,
};
use crate::interval::query::{isolate, requirements};
use crate::scoregrid;

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

pub fn home_booksum(offer: &Offer) -> f64 {
    offer
        .filter_outcomes_with_probs(|outcome_type, _| {
            matches!(
                outcome_type,
                OutcomeType::Player(Player::Named(Side::Home, _))
            )
        })
        .map(|(_, prob)| prob)
        .sum()
}

pub fn away_booksum(offer: &Offer) -> f64 {
    offer
        .filter_outcomes_with_probs(|outcome_type, _| {
            matches!(
                outcome_type,
                OutcomeType::Player(Player::Named(Side::Away, _))
            )
        })
        .map(|(_, prob)| prob)
        .sum()
}

// pub fn fit_scoregrid_half(offers: &[&Offer]) -> HypergridSearchOutcome {
//     let init_estimates = {
//         let start = Instant::now();
//         let search_outcome = fit_bivariate_poisson_scoregrid(offers, MAX_TOTAL_GOALS_HALF);
//         let elapsed = start.elapsed();
//         println!("biv-poisson: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_lambdas(&search_outcome.optimal_values));
//         search_outcome
//             .optimal_values
//             .iter()
//             .map(|optimal_value| {
//                 1.0 - poisson::univariate(
//                     0,
//                     optimal_value / INTERVALS as f64 * 2.0,
//                     &factorial::Calculator,
//                 )
//             })
//             .collect::<Vec<_>>()
//     };
//     println!("initial estimates: {init_estimates:?}");
//
//     let start = Instant::now();
//     let search_outcome = fit_bivariate_binomial_scoregrid(
//         offers,
//         &init_estimates,
//         (INTERVALS / 2) as u8,
//         MAX_TOTAL_GOALS_HALF,
//     );
//     let elapsed = start.elapsed();
//     println!("biv-binomial: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
//     search_outcome
// }

pub fn fit_scoregrid_half(
    home_goals_estimate: f64,
    away_goals_estimate: f64,
    offers: &[&Offer],
    intervals: u8,
    max_total_goals_half: u16) -> HypergridSearchOutcome {
    let init_estimates = {
        let start = Instant::now();
        let search_outcome = fit_univariate_poisson_scoregrid(home_goals_estimate, away_goals_estimate, offers, intervals, max_total_goals_half);
        let elapsed = start.elapsed();
        debug!("fitted univariate Poisson: took {elapsed:?}, search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_univariate_poisson(&search_outcome.optimal_values));
        search_outcome
            .optimal_values
            .iter()
            .map(|optimal_value| {
                1.0 - poisson::univariate(
                    0,
                    optimal_value / intervals as f64 * 2.0,
                    &factorial::Calculator,
                )
            })
            .collect::<Vec<_>>()
    };
    println!("initial estimates: {init_estimates:?}");
    HypergridSearchOutcome {
        steps: 0,
        optimal_values: init_estimates,
        optimal_residual: 0.0,
    }

    // let start = Instant::now();
    // let search_outcome = fit_univariate_binomial_scoregrid(
    //     offers,
    //     &init_estimates,
    //     (INTERVALS / 2) as u8,
    //     MAX_TOTAL_GOALS_HALF,
    // );
    // let elapsed = start.elapsed();
    // println!("biv-binomial: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
    // search_outcome
}
//
// pub fn fit_scoregrid_full(offers: &[&Offer]) -> HypergridSearchOutcome {
//     let init_estimates = {
//         println!("*** f/t: fitting bivariate poisson scoregrid ***");
//         let start = Instant::now();
//         let search_outcome = fit_bivariate_poisson_scoregrid(offers, MAX_TOTAL_GOALS_FULL);
//         let elapsed = start.elapsed();
//         println!(
//             "f/t: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}",
//             expectation_from_bivariate_poisson(&search_outcome.optimal_values)
//         );
//         search_outcome
//             .optimal_values
//             .iter()
//             .map(|optimal_value| {
//                 poisson::univariate(
//                     1,
//                     optimal_value / INTERVALS as f64,
//                     &factorial::Calculator,
//                 )
//             })
//             .collect::<Vec<_>>()
//     };
//     println!("f/t: initial estimates: {init_estimates:?}");
//
//     // HypergridSearchOutcome {
//     //     steps: 0,
//     //     optimal_values: init_estimates,
//     //     optimal_residual: 0.0,
//     // }
//
//     println!("*** f/t: fitting bivariate binomial scoregrid ***");
//     let start = Instant::now();
//     let search_outcome = fit_bivariate_binomial_scoregrid(
//         offers,
//         &init_estimates,
//         INTERVALS as u8,
//         MAX_TOTAL_GOALS_FULL,
//     );
//     // let search_outcome = fit_scoregrid(&[&correct_score]);
//     let elapsed = start.elapsed();
//     println!("f/t: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
//     search_outcome
// }

pub fn fit_scoregrid_full(h2h: &Offer, total_goals: &Offer, intervals: u8, max_total_goals: u16) -> (HypergridSearchOutcome, Vec<f64>) {
    let expected_total_goals_per_side = {
        let start = Instant::now();
        let init_estimate = match &total_goals.offer_type {
            OfferType::TotalGoals(_, over) => over.0 as f64 + 0.5,
            _ => unreachable!()
        };
        let search_outcome =
            fit_poisson_total_goals_scoregrid(init_estimate, total_goals, intervals, max_total_goals);
        let elapsed = start.elapsed();
        debug!("fitted f/t Poisson total goals ({init_estimate:.1}): took {elapsed:?}, {search_outcome:?}");
        search_outcome.optimal_value
    };

    let expected_home_goals = {
        let start = Instant::now();
        let search_outcome =
            fit_poisson_h2h_scoregrid(expected_total_goals_per_side, h2h, intervals, max_total_goals);
        let elapsed = start.elapsed();
        debug!("fitted f/t Poisson goals per side: took {elapsed:?}, {search_outcome:?}");
        search_outcome.optimal_value
    };

    let offers = &[h2h, total_goals];
    let expected_common_goals = {
        let start = Instant::now();
        let search_outcome = fit_poisson_common_scoregrid(
            expected_home_goals,
            2.0 * expected_total_goals_per_side - expected_home_goals,
            h2h,
            intervals,
            max_total_goals,
        );
        let elapsed = start.elapsed();
        debug!("fitted f/t Poisson common goals: took {elapsed:?}, {search_outcome:?}");
        search_outcome.optimal_value
    };

    let est_lambdas = [expected_home_goals - expected_common_goals, 2.0 * expected_total_goals_per_side - expected_home_goals - expected_common_goals, expected_common_goals];
    println!("est_lambdas: {est_lambdas:?}, expectation: {:.3}", expectation_from_bivariate_poisson(&est_lambdas));
    // let init_estimates = est_lambdas.iter()
    //         .map(|optimal_value| {
    //             1.0 - poisson::univariate(0, optimal_value / INTERVALS as f64, &factorial::Calculator)
    //         })
    //         .collect::<Vec<_>>();

    let (init_estimates, lambdas) = {
        let start = Instant::now();
        let expected_away_goals = 2.0 * expected_total_goals_per_side - expected_home_goals;
        let search_outcome = fit_bivariate_poisson_scoregrid(offers, expected_home_goals, expected_away_goals, expected_common_goals, intervals, max_total_goals);
        let elapsed = start.elapsed();
        debug!(
            "fitted f/t bivariate Poisson: took {elapsed:?}, {search_outcome:?}, expectation: {:.3}",
            expectation_from_bivariate_poisson(&search_outcome.optimal_values)
        );
        (search_outcome
            .optimal_values
            .iter()
            .map(|optimal_value| {
                1.0 - poisson::univariate(0, optimal_value / intervals as f64, &factorial::Calculator)
            })
            .collect::<Vec<_>>(), search_outcome.optimal_values)
    };
    println!("f/t: initial estimates: {init_estimates:?}");

    // HypergridSearchOutcome {
    //     steps: 0,
    //     optimal_values: init_estimates,
    //     optimal_residual: 0.0,
    // }

    let start = Instant::now();
    let search_outcome = fit_bivariate_binomial_scoregrid(
        offers,
        &init_estimates,
        intervals,
        max_total_goals,
    );
    // let search_outcome = fit_scoregrid(&[&correct_score]);
    let elapsed = start.elapsed();
    debug!("fitted f/t bivariate binomial: took {elapsed:?}, {search_outcome:?}");
    (search_outcome, lambdas)
}

pub fn fit_first_goalscorer_all<'a>(
    h1_probs: &'a BivariateProbs,
    h2_probs: &'a BivariateProbs,
    first_goalscorer: &'a Offer,
    nil_all_draw_prob: f64,
    intervals: u8,
    max_total_goals: u16
) -> Vec<(Player, f64)> {
    let home_rate = (h1_probs.home + h2_probs.home) / 2.0;
    let away_rate = (h1_probs.away + h2_probs.away) / 2.0;
    let common_rate = (h1_probs.common + h2_probs.common) / 2.0;
    let rate_sum = home_rate + away_rate + common_rate;
    let home_ratio = (home_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob);
    let away_ratio = (away_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob);
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let start = Instant::now();
    let probs = first_goalscorer.outcomes.items().iter().enumerate()
        .filter(|(_, outcome)| matches!(outcome, OutcomeType::Player(_)))
        .map(|(index, outcome)| {
        match outcome {
            OutcomeType::Player(player) => {
                let side_ratio = match player {
                    Named(side, _) => match side {
                        Side::Home => home_ratio,
                        Side::Away => away_ratio,
                    },
                    Player::Other => unreachable!(),
                };
                let init_estimate = first_goalscorer.market.probs[index] / side_ratio;
                // let per_start = Instant::now();
                let player_search_outcome = fit_first_goalscorer_one(
                    h1_probs,
                    h2_probs,
                    player,
                    init_estimate,
                    first_goalscorer.market.probs[index],
                    intervals,
                    max_total_goals
                );
                // println!("first goal for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}, took {:?}", first_goalscorer.market.probs[index], per_start.elapsed());
                (player.clone(), player_search_outcome.optimal_value)
            }
            _ => unreachable!(),
        }
    })
    .collect::<Vec<_>>();
    let elapsed = start.elapsed();
    debug!("first goalscorer fitting took {elapsed:?}");
    probs
}

fn fit_first_goalscorer_one(
    h1_goals: &BivariateProbs,
    h2_goals: &BivariateProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
    intervals: u8,
    max_total_goals: u16
) -> UnivariateDescentOutcome {
    // println!("size of RawArray={}", mem::size_of::<RawArray<(Player, PlayerProbs), 3>>());
    // println!("size of Explicit<RawArray>={}", mem::size_of::<Explicit<RawArray<(Player, PlayerProbs), 3>>>());
    // let mut player_probs: StackVec<(Player, PlayerProbs), 3> = StackVec::default();
    // player_probs.push((
    //     player.clone(),
    //     PlayerProbs {
    //         goal: Some(0.0),
    //         assist: None,
    //     },
    // ));
    let player_probs = sv![(
            player.clone(),
            PlayerProbs {
                goal: Some(0.0),
                assist: None,
            },
        )];
    let mut config = Config {
        intervals,
        team_probs: TeamProbs {
            h1_goals: h1_goals.clone(),
            h2_goals: h2_goals.clone(),
            assists: UnivariateProbs::default(),
        },
        player_probs,
        prune_thresholds: PruneThresholds {
            max_total_goals,
            min_prob: GOALSCORER_MIN_PROB,
        },
        expansions: requirements(&OfferType::FirstGoalscorer),
    };
    let outcome_type = OutcomeType::Player(player.clone());
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.0001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            config.player_probs[0].1.goal = Some(value);
            let exploration = explore(&config, 0..intervals);
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

pub fn fit_anytime_goalscorer_all<'a>(
    h1_probs: &'a BivariateProbs,
    h2_probs: &'a BivariateProbs,
    anytime_goalscorer: &'a Offer,
    nil_all_draw_prob: f64,
    prob_est_adj: f64,
    intervals: u8,
    max_total_goals: u16
) -> Vec<(Player, f64)> {
    let home_rate = (h1_probs.home + h2_probs.home) / 2.0;
    let away_rate = (h1_probs.away + h2_probs.away) / 2.0;
    let common_rate = (h1_probs.common + h2_probs.common) / 2.0;
    let rate_sum = home_rate + away_rate + common_rate;
    let home_ratio = (home_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob);
    let away_ratio = (away_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob);
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let start = Instant::now();
    let probs = anytime_goalscorer.outcomes.items().iter().enumerate()
        .filter(|(_, outcome)| matches!(outcome, OutcomeType::Player(_)))
        .map(|(index, outcome)| {
            match outcome {
                OutcomeType::Player(player) => {
                    let side_ratio = match player {
                        Named(side, _) => match side {
                            Side::Home => home_ratio,
                            Side::Away => away_ratio,
                        },
                        Player::Other => unreachable!(),
                    };
                    let init_estimate = anytime_goalscorer.market.probs[index] / side_ratio * prob_est_adj;
                    let player_search_outcome = fit_anytime_goalscorer_one(
                        h1_probs,
                        h2_probs,
                        player,
                        init_estimate,
                        anytime_goalscorer.market.probs[index],
                        intervals,
                        max_total_goals
                    );
                    // println!("anytime goal for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", anytime_goalscorer.market.probs[index]);
                    (player.clone(), player_search_outcome.optimal_value)
                }
                _ => unreachable!(),
            }
        })
        .collect::<Vec<_>>();
    let elapsed = start.elapsed();
    debug!("anytime goalscorer fitting took {elapsed:?}");
    probs
}

fn fit_anytime_goalscorer_one(
    h1_goals: &BivariateProbs,
    h2_goals: &BivariateProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
    intervals: u8,
    max_total_goals: u16
) -> UnivariateDescentOutcome {
    let mut config = Config {
        intervals,
        team_probs: TeamProbs {
            h1_goals: h1_goals.clone(),
            h2_goals: h2_goals.clone(),
            assists: UnivariateProbs::default(),
        },
        player_probs: sv![(
            player.clone(),
            PlayerProbs {
                goal: Some(0.0),
                assist: None,
            },
        )],
        prune_thresholds: PruneThresholds {
            max_total_goals,
            min_prob: GOALSCORER_MIN_PROB,
        },
        expansions: requirements(&OfferType::AnytimeGoalscorer),
    };
    let outcome_type = OutcomeType::Player(player.clone());
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.0001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            config.player_probs[0].1.goal = Some(value);
            let exploration = explore(&config, 0..intervals);
            let isolated_prob = isolate(
                &OfferType::AnytimeGoalscorer,
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
    assist_probs: &UnivariateProbs,
    anytime_assist: &Offer,
    nil_all_draw_prob: f64,
    booksum: f64,
    intervals: u8,
    max_total_goals: u16
) -> Vec<(Player, f64)> {
    let home_rate = (h1_probs.home + h2_probs.home) / 2.0;
    let away_rate = (h1_probs.away + h2_probs.away) / 2.0;
    let common_rate = (h1_probs.common + h2_probs.common) / 2.0;
    let rate_sum = home_rate + away_rate + common_rate;
    let home_ratio =
        (home_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob) * assist_probs.home;
    let away_ratio =
        (away_rate + common_rate / 2.0) / rate_sum * (1.0 - nil_all_draw_prob) * assist_probs.away;
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let start = Instant::now();
    let probs = anytime_assist.outcomes.items().iter().enumerate().map(|(index, outcome)| {
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
                    assist_probs,
                    player,
                    init_estimate,
                    anytime_assist.market.probs[index],
                    intervals,
                    max_total_goals
                );
                // println!("assist for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", anytime_assist.market.probs[index]);
                (player.clone(), player_search_outcome.optimal_value)
            }
            _ => unreachable!(),
        }
    }).collect::<Vec<_>>();
    let elapsed = start.elapsed();
    debug!("anytime assist fitting took {elapsed:?}");
    probs
}

fn fit_anytime_assist_one(
    h1_goals: &BivariateProbs,
    h2_goals: &BivariateProbs,
    assist_probs: &UnivariateProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
    intervals: u8,
    max_total_goals: u16
) -> UnivariateDescentOutcome {
    let mut config = Config {
        intervals,
        team_probs: TeamProbs {
            h1_goals: h1_goals.clone(),
            h2_goals: h2_goals.clone(),
            assists: assist_probs.clone(),
        },
        player_probs: sv![(
            player.clone(),
            PlayerProbs {
                goal: None,
                assist: Some(0.0),
            },
        )],
        prune_thresholds: PruneThresholds {
            max_total_goals,
            min_prob: GOALSCORER_MIN_PROB,
        },
        expansions: requirements(&OfferType::AnytimeAssist),
    };
    let outcome_type = OutcomeType::Player(player.clone());
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.0001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            config.player_probs[0].1.assist = Some(value);
            let exploration = explore(&config, 0..intervals);
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

fn expectation_from_univariate_poisson(lambdas: &[f64]) -> f64 {
    assert_eq!(2, lambdas.len());
    lambdas[0] + lambdas[1]
}

fn expectation_from_bivariate_poisson(lambdas: &[f64]) -> f64 {
    assert_eq!(3, lambdas.len());
    lambdas[0] + lambdas[1] + 2.0 * lambdas[2]
}

fn allocate_scoregrid(intervals: u8, max_total_goals: u16) -> Matrix<f64> {
    let dim = usize::min(max_total_goals as usize, intervals as usize) + 1;
    Matrix::allocate(dim, dim)
}

fn fit_poisson_total_goals_scoregrid(
    init_estimate: f64,
    total_goals: &Offer,
    intervals: u8,
    max_total_goals: u16,
) -> UnivariateDescentOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    let offers = [total_goals];
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: 0.0001,
            max_steps: 100,
            acceptable_residual: 1e-6,
        },
        |value| {
            univariate_poisson_scoregrid(value, value, &mut scoregrid);
            scoregrid_error(&offers, &scoregrid)
        },
    )
}

fn fit_poisson_h2h_scoregrid(
    init_estimate: f64,
    h2h: &Offer,
    intervals: u8,
    max_total_goals: u16,
) -> UnivariateDescentOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    let offers = [h2h];
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: 0.0001,
            max_steps: 100,
            acceptable_residual: 1e-6,
        },
        |value| {
            univariate_poisson_scoregrid(value, 2.0 * init_estimate - value, &mut scoregrid);
            scoregrid_error(&offers, &scoregrid)
        },
    )
}

fn fit_poisson_common_scoregrid(
    home_goals_estimate: f64,
    away_goals_estimate: f64,
    h2h: &Offer,
    intervals: u8,
    max_total_goals: u16,
) -> UnivariateDescentOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    let offers = [h2h];
    univariate_descent(&UnivariateDescentConfig {
        init_value: 0.0,
        init_step: 0.1,
        min_step: 0.0001,
        max_steps: 100,
        acceptable_residual: 1e-6,
    }, |value| {
        bivariate_poisson_scoregrid(home_goals_estimate - value, away_goals_estimate - value, value, &mut scoregrid);
        scoregrid_error(&offers, &scoregrid)
    })
    // let bounds = [0.01..=0.5];
    // hypergrid_search(
    //     &HypergridSearchConfig {
    //         max_steps: 100,
    //         acceptable_residual: 1e-6,
    //         bounds: Capture::Borrowed(&bounds),
    //         resolution: 100,
    //     },
    //     |_| true,
    //     |values| {
    //         bivariate_poisson_scoregrid(
    //             home_goals - values[0],
    //             away_goals - values[0],
    //             values[0],
    //             &mut scoregrid,
    //         );
    //         scoregrid_error(&offers, &scoregrid)
    //     },
    // )
}

fn fit_univariate_poisson_scoregrid(
    home_goals_estimate: f64,
    away_goals_estimate: f64,
    offers: &[&Offer],
    intervals: u8,
    max_total_goals: u16,
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    let bounds = [home_goals_estimate * 0.83..=home_goals_estimate * 1.2, away_goals_estimate * 0.83..=away_goals_estimate * 1.20];
    // let bounds = [0.2..=3.0, 0.2..=3.0];
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: Capture::Borrowed(&bounds),
            resolution: 10,
        },
        |_| true,
        |values| {
            univariate_poisson_scoregrid(values[0], values[1], &mut scoregrid);
            scoregrid_error(offers, &scoregrid)
        },
    )
}

fn fit_bivariate_poisson_scoregrid(
    offers: &[&Offer],
    home_estimate: f64,
    away_estimate: f64,
    common_estimate: f64,
    intervals: u8,
    max_total_goals: u16,
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    // println!("estimates: {home_estimate} and {away_estimate}");
    let bounds = [home_estimate - 0.5..=home_estimate, away_estimate - 0.5..=away_estimate, common_estimate..=common_estimate + 0.5];
    // let bounds = [0.2..=3.0, 0.2..=3.0, 0.0..=0.5];
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: Capture::Borrowed(&bounds),
            resolution: 4,
        },
        |_| true,
        |values| {
            bivariate_poisson_scoregrid(values[0], values[1], values[2], &mut scoregrid);
            scoregrid_error(offers, &scoregrid)
        },
    )
}

fn fit_univariate_binomial_scoregrid(
    offers: &[&Offer],
    init_estimates: &[f64],
    intervals: u8,
    max_total_goals: u16,
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
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
            univariate_binomial_scoregrid(intervals, values[0], values[1], &mut scoregrid);
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
    let mut scoregrid = allocate_scoregrid(intervals, max_total_goals);
    let bounds = init_estimates
        .iter()
        .map(|&estimate| (estimate * 0.67)..=(estimate * 1.5))
        .collect::<Vec<_>>();
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: Capture::Borrowed(&bounds),
            resolution: 4,
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

/// Univariate Poisson.
fn univariate_poisson_scoregrid(home_rate: f64, away_rate: f64, scoregrid: &mut Matrix<f64>) {
    scoregrid.fill(0.0);
    scoregrid::from_univariate_poisson(home_rate, away_rate, scoregrid);
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
}

/// Univariate binomial.
fn univariate_binomial_scoregrid(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_binomial(intervals, interval_home_prob, interval_away_prob, scoregrid);
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

#[derive(Debug)]
pub struct FittingErrors {
    pub rmse: f64,
    pub rmsre: f64,
}
