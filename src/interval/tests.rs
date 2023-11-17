use super::*;

#[test]
fn explore_all_2x2() {
    let exploration = explore_all(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        home_scorers: other_player(),
        away_scorers: other_player(),
    });
    assert_eq!(9, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                scorers: Default::default(),
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 3)
                ]),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 4)
                ]),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 3)
                ]),
            },
            0.125,
        ),
    ];
    assert_eq!(expected.len(), exploration.prospects.len());
    assert_eq!(0.0, exploration.pruned);
    for (expected_prospect, expected_probability) in expected {
        assert_eq!(
            &expected_probability,
            exploration.prospects.get(&expected_prospect).expect(&format!("missing {expected_prospect:?}"))
        );
    }
}

#[test]
fn explore_all_2x2_pruned() {
    let exploration = explore_all(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: 2,
        home_scorers: other_player(),
        away_scorers: other_player(),
    });
    let expected = [
        (
            Prospect {
                score: Score { home: 0, away: 0 },
                scorers: Default::default()
            },
            0.0625f64,
        ),
        (
            Prospect {
                score: Score { home: 2, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.25,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 2 },
                scorers: BTreeMap::from([
                    (Player::Other, 2)
                ]),
            },
            0.0625,
        ),
        (
            Prospect {
                score: Score { home: 1, away: 0 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
            },
            0.125,
        ),
        (
            Prospect {
                score: Score { home: 0, away: 1 },
                scorers: BTreeMap::from([
                    (Player::Other, 1)
                ]),
            },
            0.125,
        ),
    ];
    println!("exploration: {exploration:?}");
    assert_eq!(1.0 - 0.3125, exploration.prospects.values().sum::<f64>());
    assert_eq!(expected.len(), exploration.prospects.len());
    assert_eq!(0.3125, exploration.pruned);
    for (expected_prospect, expected_probability) in expected {
        assert_eq!(
            &expected_probability,
            exploration.prospects.get(&expected_prospect).expect(&format!("missing {expected_prospect:?}"))
        );
    }
}

#[test]
fn explore_all_3x3() {
    let exploration = explore_all(&IntervalConfig {
        intervals: 3,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        home_scorers: other_player(),
        away_scorers: other_player(),
    });
    assert_eq!(16, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}

#[test]
fn explore_all_4x4() {
    let exploration = explore_all(&IntervalConfig {
        intervals: 4,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
        home_scorers: other_player(),
        away_scorers: other_player(),
    });
    assert_eq!(25, exploration.prospects.len());
    assert_eq!(1.0, exploration.prospects.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}