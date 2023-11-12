use super::*;

#[test]
fn explore_all_2x2() {
    let exploration = explore_all(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
        max_total_goals: u16::MAX,
    });
    assert_eq!(9, exploration.scenarios.len());
    assert_eq!(1.0, exploration.scenarios.values().sum::<f64>());
    let expected = [
        (
            Scenario {
                score: Score { home: 0, away: 0 },
            },
            0.0625f64,
        ),
        (
            Scenario {
                score: Score { home: 2, away: 0 },
            },
            0.0625,
        ),
        (
            Scenario {
                score: Score { home: 1, away: 1 },
            },
            0.25,
        ),
        (
            Scenario {
                score: Score { home: 1, away: 2 },
            },
            0.125,
        ),
        (
            Scenario {
                score: Score { home: 0, away: 2 },
            },
            0.0625,
        ),
        (
            Scenario {
                score: Score { home: 2, away: 2 },
            },
            0.0625,
        ),
        (
            Scenario {
                score: Score { home: 1, away: 0 },
            },
            0.125,
        ),
        (
            Scenario {
                score: Score { home: 0, away: 1 },
            },
            0.125,
        ),
        (
            Scenario {
                score: Score { home: 2, away: 1 },
            },
            0.125,
        ),
    ];
    assert_eq!(expected.len(), exploration.scenarios.len());
    assert_eq!(0.0, exploration.pruned);
    for (expected_scenario, expected_probability) in expected {
        assert_eq!(
            &expected_probability,
            exploration.scenarios.get(&expected_scenario).unwrap()
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
    });
    let expected = [
        (
            Scenario {
                score: Score { home: 0, away: 0 },
            },
            0.0625f64,
        ),
        (
            Scenario {
                score: Score { home: 2, away: 0 },
            },
            0.0625,
        ),
        (
            Scenario {
                score: Score { home: 1, away: 1 },
            },
            0.25,
        ),
        (
            Scenario {
                score: Score { home: 0, away: 2 },
            },
            0.0625,
        ),
        (
            Scenario {
                score: Score { home: 1, away: 0 },
            },
            0.125,
        ),
        (
            Scenario {
                score: Score { home: 0, away: 1 },
            },
            0.125,
        ),
    ];
    assert_eq!(1.0 - 0.3125, exploration.scenarios.values().sum::<f64>());
    assert_eq!(expected.len(), exploration.scenarios.len());
    assert_eq!(0.3125, exploration.pruned);
    for (expected_scenario, expected_probability) in expected {
        assert_eq!(
            &expected_probability,
            exploration.scenarios.get(&expected_scenario).expect(&format!("missing {expected_scenario:?}"))
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
    });
    assert_eq!(16, exploration.scenarios.len());
    assert_eq!(1.0, exploration.scenarios.values().sum::<f64>());
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
    });
    assert_eq!(25, exploration.scenarios.len());
    assert_eq!(1.0, exploration.scenarios.values().sum::<f64>());
    assert_eq!(0.0, exploration.pruned);
}