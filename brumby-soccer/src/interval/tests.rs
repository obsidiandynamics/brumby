use super::*;
use crate::domain::{OfferType, OutcomeType, Player};
use assert_float_eq::*;
use crate::interval::query::isolate;

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
        &IntervalConfig {
            intervals: 2,
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }}],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 2}, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0}, h2: SplitStats { goals: 2 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 2 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 2 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 2 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 2 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 2 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 2 }, h2: SplitStats { goals: 2 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 2 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 2 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, isolated_1gs_other);

    let isolated_anytime_none = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_anytime_none);

    let isolated_anytime_other = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.0625, isolated_anytime_other);
}

#[test]
fn explore_2x2_pruned_2_goals() {
    let exploration = explore(
        &IntervalConfig {
            intervals: 2,
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![],
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
                score: Score { home: 0, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }}],
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 2}, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0}, h2: SplitStats { goals: 2 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 1 }, h2: SplitStats { goals: 0 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}],
                first_scorer: Some(0),
            },
            0.0625,
        ),
    ];
    assert_eq!(1.0 - 0.3125, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.3125, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(
        &OfferType::FirstGoalscorer,
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
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..3,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(32, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_4x4() {
    let exploration = explore(
        &IntervalConfig {
            intervals: 4,
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..4,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(65, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_1x1_player() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(
        &IntervalConfig {
            intervals: 1,
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![(player.clone(), 0.25)],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..1,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }}
                ],
                first_scorer: None,
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 2 }}
                ],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}
                ],
                first_scorer: Some(1),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}
                ],
                first_scorer: Some(0),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}
                ],
                first_scorer: Some(1),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }}
                ],
                first_scorer: Some(0),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                stats: vec![
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 0 }},
                    PlayerStats { h1: SplitStats { goals: 0 }, h2: SplitStats { goals: 1 }}
                ],
                first_scorer: Some(1),
            },
            0.25,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.09375, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(1.0 - 0.25 - 0.09375, isolated_1gs_other);

    let isolated_anytime_none = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.25, isolated_anytime_none);

    let isolated_anytime_player = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.125, isolated_anytime_player);

    let isolated_anytime_other = isolate(
        &OfferType::AnytimeGoalscorer,
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
            h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
            players: vec![(player.clone(), 0.25)],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let isolated_1gs_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_eq!(0.1171875, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &OfferType::FirstGoalscorer,
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
            h1_probs: ScoringProbs { home_prob: 0.3, away_prob: 0.2, common_prob: 0.1 },
            h2_probs: ScoringProbs { home_prob: 0.3, away_prob: 0.2, common_prob: 0.1 },
            players: vec![(player.clone(), 0.25)],
            prune_thresholds: PruneThresholds {
                max_total_goals: u16::MAX,
                min_prob: 0.0,
            },
            expansions: Default::default(),
        },
        0..2,
    );
    print_prospects(&exploration.prospects);
    assert_float_relative_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);

    let isolated_1gs_none = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::None,
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.16, isolated_1gs_none);

    let isolated_1gs_player = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(0.1225, isolated_1gs_player);

    let isolated_1gs_other = isolate(
        &OfferType::FirstGoalscorer,
        &OutcomeType::Player(Player::Other),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert_float_relative_eq!(1.0 - 0.16 - 0.1225, isolated_1gs_other);
}
