use super::*;
use crate::entity::Player;
use assert_float_eq::*;

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
        &IntervalConfig {
            intervals: 2,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: u16::MAX,
            players: vec![],
        },
        0..2,
    );
    assert_eq!(9, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                stats: vec![PlayerStats { goals: 0 }],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 2 },
                stats: vec![PlayerStats { goals: 3 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 2 },
                stats: vec![PlayerStats { goals: 4 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { goals: 1 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { goals: 1 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 1 },
                stats: vec![PlayerStats { goals: 3 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, isolated_1gs_other);

    let isolated_anytime_none = isolate(
        &MarketType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_anytime_none);

    let isolated_anytime_other = isolate(
        &MarketType::AnytimeGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, isolated_anytime_other);
}

#[test]
fn explore_2x2_pruned() {
    let exploration = explore(
        &IntervalConfig {
            intervals: 2,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: 2,
            players: vec![],
        },
        0..2,
    );
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                stats: vec![PlayerStats { goals: 0 }],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                stats: vec![PlayerStats { goals: 2 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { goals: 1 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { goals: 1 }],
                first_scorer: Some(0),
            },
            0.125,
        ),
    ];
    assert_eq!(1.0 - 0.3125, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.3125, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625 - 0.3125, isolated_1gs_other);
}

#[test]
fn explore_3x3() {
    let exploration = explore(
        &IntervalConfig {
            intervals: 3,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: u16::MAX,
            players: vec![],
        },
        0..3,
    );
    assert_eq!(16, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_4x4() {
    let exploration = explore(
        &IntervalConfig {
            intervals: 4,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: u16::MAX,
            players: vec![],
        },
        0..4,
    );
    assert_eq!(25, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_1x1_player() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &IntervalConfig {
            intervals: 1,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: u16::MAX,
            players: vec![(player.clone(), 0.25)],
        },
        0..1,
    );
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                stats: vec![PlayerStats { goals: 0 }, PlayerStats { goals: 0 }],
                first_scorer: None,
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { goals: 0 }, PlayerStats { goals: 2 }],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { goals: 1 }, PlayerStats { goals: 1 }],
                first_scorer: Some(1),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { goals: 1 }, PlayerStats { goals: 1 }],
                first_scorer: Some(0),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { goals: 0 }, PlayerStats { goals: 1 }],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { goals: 1 }, PlayerStats { goals: 0 }],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { goals: 0 }, PlayerStats { goals: 1 }],
                first_scorer: Some(1),
            },
            0.25,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.09375, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.25 - 0.09375, isolated_1gs_other);

    let isolated_anytime_none = isolate(
        &MarketType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, isolated_anytime_none);

    let isolated_anytime_player = isolate(
        &MarketType::AnytimeGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.125, isolated_anytime_player);

    let isolated_anytime_other = isolate(
        &MarketType::AnytimeGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.6875, isolated_anytime_other);
}

#[test]
fn explore_2x2_player() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &IntervalConfig {
            intervals: 2,
            h1_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_params: ModelParams { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            max_total_goals: u16::MAX,
            players: vec![(player.clone(), 0.25)],
        },
        0..2,
    );
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let isolated_1gs_none = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.1171875, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625 - 0.1171875, isolated_1gs_other);
}

#[test]
fn explore_2x2_player_asymmetric() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &IntervalConfig {
            intervals: 2,
            h1_params: ModelParams { home_prob: 0.3, away_prob: 0.2, common_prob: 0.1 },
            h2_params: ModelParams { home_prob: 0.3, away_prob: 0.2, common_prob: 0.1 },
            max_total_goals: u16::MAX,
            players: vec![(player.clone(), 0.25)],
        },
        0..2,
    );
    assert_float_relative_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let isolated_1gs_none = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.16, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.1225, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &MarketType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(1.0 - 0.16 - 0.1225, isolated_1gs_other);
}
