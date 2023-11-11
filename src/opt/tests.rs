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
        max_steps: 10,
        acceptable_residual: 1e-9,
        bounds: Capture::Owned(vec![0.0..=10.0, -10.0..=10.0, 0.0..=10.0]),
        resolution: 4,
    };
    // search for root of (x - 5)(x + 6)(x - 10) = 0
    let outcome = hypergrid_search(&config, |values| {
        (values[0] - 5.0).powi(2) + (values[1] + 6.0).powi(2) + (values[2] - 10.0).powi(2)
    });
    println!("outcome: {outcome:?}");
}