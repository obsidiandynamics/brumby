use super::*;
use assert_float_eq::*;

#[test]
fn univariate_descent_sqrt() {
    let config = UnivariateDescentConfig {
        init_value: 0.0,
        init_step: 0.1,
        min_step: 0.00001,
        max_steps: 100,
        acceptable_residual: 0.0
    };
    let outcome = univariate_descent(&config, |value| (81.0 - value.powi(2)).powi(2));
    assert_float_absolute_eq!(9.0, outcome.optimal_value, config.min_step);
}

#[test]
fn hypergrid_search_poly3() {
    let config = HypergridSearchConfig {
        max_steps: 100,
        acceptable_residual: 1e-12,
        bounds: Capture::Owned(vec![0.0..=10.0, -10.0..=10.0, 0.0..=10.0]),
        resolution: 4,
    };
    // search for root of (x - 5)(x + 6)(x - 10) = 0
    let outcome = hypergrid_search::<3>(&config, |_| true, |values| {
        (values[0] - 5.0).powi(2) + (values[1] + 6.0).powi(2) + (values[2] - 10.0).powi(2)
    });
    println!("outcome: {outcome:?}");
    assert_float_absolute_eq!(5.0, outcome.optimal_values[0]);
    assert_float_absolute_eq!(-6.0, outcome.optimal_values[1]);
    assert_float_absolute_eq!(10.0, outcome.optimal_values[2]);
}