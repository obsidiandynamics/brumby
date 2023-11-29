use super::*;

#[test]
fn explore_all_2x2() {
    let scenarios = explore_all(&IntervalConfig {
        intervals: 2,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
    });
    assert_eq!(9, scenarios.len());
    assert_eq!(1.0, scenarios.values().sum::<f64>());
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
    assert_eq!(expected.len(), scenarios.len());
    for (expected_scenario, expected_probability) in expected {
        assert_eq!(
            &expected_probability,
            scenarios.get(&expected_scenario).unwrap()
        );
    }
}

#[test]
fn explore_all_3x3() {
    let scenarios = explore_all(&IntervalConfig {
        intervals: 3,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
    });
    assert_eq!(16, scenarios.len());
    assert_eq!(1.0, scenarios.values().sum::<f64>());
}

#[test]
fn explore_all_4x4() {
    let scenarios = explore_all(&IntervalConfig {
        intervals: 4,
        home_prob: 0.25,
        away_prob: 0.25,
        common_prob: 0.25,
    });
    assert_eq!(25, scenarios.len());
    assert_eq!(1.0, scenarios.values().sum::<f64>());
}