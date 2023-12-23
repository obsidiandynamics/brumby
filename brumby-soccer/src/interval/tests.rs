use assert_float_eq::*;
use brumby::sv;

use crate::domain::{OfferType, OutcomeType, Player};
use crate::interval::query::isolate;

use super::*;

// #[macro_export]
// macro_rules! sv {
//     () => (
//         StackVec4::default()
//     );
//     ( $( $x:expr ),* ) => {
//         {
//             let mut sv = StackVec4::default();
//             $(
//                 sv.push($x);
//             )*
//             sv
//         }
//     };
// }

fn print_prospects(prospects: &Prospects) {
    for (prospect, prob) in prospects {
        println!("prospect: {prospect:?} @ {prob}");
    }
}

fn assert_expected_prospects(expected: &[(Prospect, f64)], actual: &Prospects) {
    for (expected_prospect, expected_probability) in expected {
        let actual_probability = actual
            .get(&expected_prospect)
            .expect(&format!("missing {expected_prospect:?}"));
        assert_eq!(
            expected_probability, actual_probability,
            "for expected {expected_prospect:?}"
        );
    }
    assert_eq!(expected.len(), actual.len());
}

#[test]
fn explore_2x2() {
    let exploration = explore(
        &Config {
            intervals: 2,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 }],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 2, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 1 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 2 }, h2: PeriodStats { goals: 0 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0}, h2: PeriodStats { goals: 2 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 1 },
                ft_score: Score { home: 1, away: 2 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 2 }, h2: PeriodStats { goals: 1 }, assists: 3 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 1 },
                ft_score: Score { home: 1, away: 2 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 2 }, assists: 3 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 1 },
                ft_score: Score { home: 0, away: 2 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 1 },
                ft_score: Score { home: 2, away: 2 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 2 }, h2: PeriodStats { goals: 2 }, assists: 4 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 0 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 1 },
                ft_score: Score { home: 0, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 0 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 1 },
                ft_score: Score { home: 2, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 2 }, h2: PeriodStats { goals: 1 }, assists: 3 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 2, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 2 }, assists: 3 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let first_goalscorer_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, first_goalscorer_none);

    let first_goalscorer_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, first_goalscorer_other);

    let anytime_goalscorer_none = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, anytime_goalscorer_none);

    let anytime_goalscorer_other = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, anytime_goalscorer_other);

    let anytime_assist_none = isolate(
        &OfferType::AnytimeAssist,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, anytime_assist_none);

    let anytime_assist_other = isolate(
        &OfferType::AnytimeAssist,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, anytime_assist_other);
}

#[test]
fn explore_2x2_pruned_2_goals() {
    let exploration = explore(
        &Config {
            intervals: 2,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![],
            prune_thresholds: PruneThresholds {
                max_total_goals: 2,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    let expected = [
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 }],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 2, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 1 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 2 }, h2: PeriodStats { goals: 0 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0}, h2: PeriodStats { goals: 2 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 1 },
                ft_score: Score { home: 0, away: 2 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 1 }, assists: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 1, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 0 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 1 },
                ft_score: Score { home: 0, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 1 }, h2: PeriodStats { goals: 0 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 1 },
                stats:sv![PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
    ];
    assert_eq!(1.0 - 0.3125, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.3125, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let first_goalscorer_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, first_goalscorer_none);

    let fist_goalscorer_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625 - 0.3125, fist_goalscorer_other);
}

#[test]
fn explore_3x3() {
    let exploration = explore(
        &Config {
            intervals: 3,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..3,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(36, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_4x4() {
    let exploration = explore(
        &Config {
            intervals: 4,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..4,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(81, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_1x1_player_goal() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &Config {
            intervals: 1,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![(player.clone(), PlayerProbs { goal: Some(0.25), assist: None })],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..1,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 0 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 }
                ],
                first_scorer: None,
            },
            0.25,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 2 }, assists: 2 }
                ],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 2 }
                ],
                first_scorer: Some(1),
            },
            0.03125,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 1 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 2 }
                ],
                first_scorer: Some(0),
            },
            0.03125,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }
                ],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 1, away: 0 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 1 }
                ],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                ht_score: Score { home: 0, away: 0 },
                ft_score: Score { home: 0, away: 1 },
                stats:sv![
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 0 }, assists: 0 },
                    PlayerStats { h1: PeriodStats { goals: 0 }, h2: PeriodStats { goals: 1 }, assists: 1 }
                ],
                first_scorer: Some(1),
            },
            0.25,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let first_goalscorer_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, first_goalscorer_none);

    let first_goalscorer_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.09375, first_goalscorer_player);

    let first_goalscorer_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.25 - 0.09375, first_goalscorer_other);

    let anytime_goalscorer_none = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, anytime_goalscorer_none);

    let anytime_goalscorer_player = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.125, anytime_goalscorer_player);

    let anytime_goalscorer_other = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.6875, anytime_goalscorer_other);

    let anytime_assist_none = isolate(
        &OfferType::AnytimeAssist,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, anytime_assist_none);

    let anytime_assist_player = isolate(
        &OfferType::AnytimeAssist,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0, anytime_assist_player);

    let anytime_assist_other = isolate(
        &OfferType::AnytimeAssist,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.25, anytime_assist_other);
}

#[test]
fn explore_2x2_player_goal() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &Config {
            intervals: 2,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                assists: UnivariateProbs { home: 0.5, away: 0.5 },
            },
            player_probs: vec![(player.clone(), PlayerProbs { goal: Some(0.25), assist: None })],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let first_goalscorer_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, first_goalscorer_none);

    let first_goalscorer_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.1171875, first_goalscorer_player);

    let first_goalscorer_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625 - 0.1171875, first_goalscorer_other);
}

#[test]
fn explore_2x2_player_goal_asymmetric() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &Config {
            intervals: 2,
            team_probs: TeamProbs {
                h1_goals: BivariateProbs { home: 0.3, away: 0.2, common: 0.1 },
                h2_goals: BivariateProbs { home: 0.3, away: 0.2, common: 0.1 },
                assists: UnivariateProbs { home: 1.0, away: 1.0 },
            },
            player_probs: vec![(player.clone(), PlayerProbs { goal: Some(0.25), assist: None })],
            prune_thresholds: Default::default(),
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_float_relative_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let first_goalscorer_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.16, first_goalscorer_none);

    let first_goalscorer_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.1225, first_goalscorer_player);

    let first_goalscorer_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(1.0 - 0.16 - 0.1225, first_goalscorer_other);
}
