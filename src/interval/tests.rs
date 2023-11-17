use crate::entity::Player;
use super::*;

fn assert_expected_prospects(expected: &[(Prospect, f64)], actual: &Prospects) {
    for (expected_prospect, expected_probability) in expected {
        let actual_probability = actual.get(&expected_prospect).expect(&format!("missing {expected_prospect:?}"));
        assert_eq!(
            expected_probability,
            actual_probability,
            "for expected {expected_prospect:?}"
        );
    }
    assert_eq!(expected.len(), actual.len());
}

#[test]
fn explore_2x2() {
    let exploration = explore(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        scorers: vec![],
    });
    assert_eq!(9, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                scorers: Default::default(),
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 3)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 4)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 3)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(&MarketType::FirstGoalscorer, &OutcomeType::None, &exploration.prospects);
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(Player::Other), &exploration.prospects);
    assert_eq!(1.0 - 0.0625, isolated_1gs_other);

    let isolated_anytime_none = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::None, &exploration.prospects);
    assert_eq!(0.0625, isolated_anytime_none);

    let isolated_anytime_other = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::Player(Player::Other), &exploration.prospects);
    assert_eq!(1.0 - 0.0625, isolated_anytime_other);
}

#[test]
fn explore_2x2_pruned() {
    let exploration = explore(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: 2,
        scorers: vec![],
    });
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                scorers: Default::default(),
                first_scorer: None,
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.125,
        ),
    ];
    assert_eq!(1.0 - 0.3125, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.3125, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(&MarketType::FirstGoalscorer, &OutcomeType::None, &exploration.prospects);
    assert_eq!(0.0625, isolated_1gs_none);

    let isolated_1gs_other = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(Player::Other), &exploration.prospects);
    assert_eq!(1.0 - 0.0625 - 0.3125, isolated_1gs_other);
}

#[test]
fn explore_3x3() {
    let exploration = explore(&IntervalConfig {
        intervals: 3,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        scorers: vec![],
    });
    assert_eq!(16, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_4x4() {
    let exploration = explore(&IntervalConfig {
        intervals: 4,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        scorers: vec![],
    });
    assert_eq!(25, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_1x1_player() {
    let player = Player::Named(Side::Home, "Markos".into());
    let exploration = explore(&IntervalConfig {
        intervals: 1,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        scorers: vec![(player.clone(), 0.25)],
    });
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                scorers: Default::default(),
                first_scorer: None,
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 2),
                ]),
                first_scorer: Some(Player::Other),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1),
                    (player.clone(), 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1),
                    (player.clone(), 1)
                ]),
                first_scorer: Some(player.clone()),
            },
            0.03125,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.1875,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (player.clone(), 1)
                ]),
                first_scorer: Some(player.clone()),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
                first_scorer: Some(Player::Other),
            },
            0.25,
        ),
    ];
    assert_eq!(0.0, exploration.pruned);
    assert_expected_prospects(&expected, &exploration.prospects);

    let isolated_1gs_none = isolate(&MarketType::FirstGoalscorer, &OutcomeType::None, &exploration.prospects);
    assert_eq!(0.25, isolated_1gs_none);

    let isolated_1gs_player = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(player.clone()), &exploration.prospects);
    assert_eq!(0.09375, isolated_1gs_player);

    let isolated_1gs_other = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(Player::Other), &exploration.prospects);
    assert_eq!(1.0 - 0.25 - 0.09375, isolated_1gs_other);

    let isolated_anytime_none = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::None, &exploration.prospects);
    assert_eq!(0.25, isolated_anytime_none);

    let isolated_anytime_player = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::Player(player.clone()), &exploration.prospects);
    assert_eq!(0.125, isolated_anytime_player);

    let isolated_anytime_other = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::Player(Player::Other), &exploration.prospects);
    assert_eq!(0.6875, isolated_anytime_other);
}