use super::*;
use crate::domain::Side;
use crate::opt::{hypergrid_search, HypergridSearchConfig};
use crate::probs::SliceExt;

#[test]
pub fn iterate_scoregrid_5x5() {
    const INTERVALS: usize = 4;
    let space = ScoreOutcomeSpace {
        interval_home_prob: 0.25,
        interval_away_prob: 0.2,
        interval_common_prob: 0.0,
    };
    let mut fixtures = IterFixtures::new(INTERVALS);
    let iter = Iter::new(&space, &mut fixtures);
    for outcome in iter {
        println!("outcome: {outcome:?}");
    }

    let mut matrix = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    let iter = Iter::new(&space, &mut fixtures);
    from_iterator(iter, &mut matrix);
    println!("matrix:\n{}", matrix.verbose());
    println!("sum: {}", matrix.flatten().sum());
}

fn create_test_4x4_scoregrid() -> Matrix<f64> {
    let mut scoregrid = Matrix::allocate(4, 4);
    scoregrid[0].copy_from_slice(&[0.04, 0.03, 0.02, 0.01]);
    scoregrid[1].copy_from_slice(&[0.08, 0.06, 0.04, 0.02]);
    scoregrid[2].copy_from_slice(&[0.12, 0.09, 0.06, 0.03]);
    scoregrid[3].copy_from_slice(&[0.16, 0.12, 0.08, 0.04]);
    scoregrid
}

#[test]
pub fn outcome_win_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.65, OutcomeType::Win(Side::Home).gather(&scoregrid));
    assert_eq!(0.15, OutcomeType::Win(Side::Away).gather(&scoregrid));
}

#[test]
pub fn outcome_draw_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.2, OutcomeType::Draw.gather(&scoregrid));
}

#[test]
pub fn outcome_goals_ou_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.35, OutcomeType::Under(3).gather(&scoregrid));
    assert_eq!(0.65, OutcomeType::Over(2).gather(&scoregrid));
}

#[test]
pub fn outcome_correct_score_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(
        0.04,
        OutcomeType::Score(Score::new(0, 0)).gather(&scoregrid)
    );
    assert_eq!(
        0.08,
        OutcomeType::Score(Score::new(3, 2)).gather(&scoregrid)
    );
}

#[test]
pub fn interval() {
    const INTERVALS: usize = 2;
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_interval(
        INTERVALS as u8,
        0..INTERVALS as u8,
        u16::MAX,
        ScoringProbs {
            home_prob: 0.25,
            away_prob: 0.25,
            common_prob: 0.25,
        },
        ScoringProbs {
            home_prob: 0.25,
            away_prob: 0.25,
            common_prob: 0.25,
        },
        &mut scoregrid,
    );
    println!(
        "scoregrid:\n{}sum: {}",
        scoregrid.verbose(),
        scoregrid.flatten().sum()
    );
}

#[test]
pub fn univariate_poisson_binomial_similarity() {
    const HOME_RATE: f64 = 1.2;
    const AWAY_RATE: f64 = 1.8;
    const INTERVALS: usize = 6;
    let mut poisson = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_univariate_poisson(HOME_RATE, AWAY_RATE, &mut poisson);
    println!(
        "poisson:\n{}sum: {}",
        poisson.verbose(),
        poisson.flatten().sum()
    );

    let interval_home_prob_est =
        1.0 - poisson::univariate(0, HOME_RATE / INTERVALS as f64, &factorial::Calculator);
    let interval_away_prob_est =
        1.0 - poisson::univariate(0, AWAY_RATE / INTERVALS as f64, &factorial::Calculator);
    println!("estimated home_prob: {interval_home_prob_est}, away_prob: {interval_away_prob_est}");
    let search_outcome = hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: vec![
                interval_home_prob_est * 0.67..=interval_home_prob_est * 1.5,
                interval_away_prob_est * 0.67..=interval_away_prob_est * 1.5,
            ]
            .into(),
            resolution: 4,
        },
        |_| true,
        |values| {
            let mut binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
            from_binomial(INTERVALS as u8, values[0], values[1], &mut binomial);
            compute_mse(poisson.flatten(), binomial.flatten())
        },
    );
    println!("search_outcome: {search_outcome:?}");

    let mut binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_binomial(
        INTERVALS as u8,
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
        &mut binomial,
    );
    println!(
        "binomial:\n{}sum: {}",
        binomial.verbose(),
        binomial.flatten().sum()
    );

    let mse = compute_mse(poisson.flatten(), binomial.flatten());
    assert!(mse < 1e-3, "mse: {mse}");
}

#[test]
pub fn bivariate_poisson_binomial_similarity() {
    const HOME_RATE: f64 = 0.8;
    const AWAY_RATE: f64 = 1.4;
    const COMMON_RATE: f64 = 0.6;
    const INTERVALS: usize = 6;
    let mut biv_poisson = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_bivariate_poisson(HOME_RATE, AWAY_RATE, COMMON_RATE, &mut biv_poisson);
    println!(
        "biv_poisson:\n{}sum: {}",
        biv_poisson.verbose(),
        biv_poisson.flatten().sum()
    );

    let interval_home_prob_est =
        1.0 - poisson::univariate(0, HOME_RATE / INTERVALS as f64, &factorial::Calculator);
    let interval_away_prob_est =
        1.0 - poisson::univariate(0, AWAY_RATE / INTERVALS as f64, &factorial::Calculator);
    let interval_common_prob_est =
        1.0 - poisson::univariate(0, COMMON_RATE / INTERVALS as f64, &factorial::Calculator);
    println!("estimated home_prob: {interval_home_prob_est}, away_prob: {interval_away_prob_est}, common_prob: {interval_common_prob_est}");
    let search_outcome = hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: vec![
                interval_home_prob_est * 0.67..=interval_home_prob_est * 1.5,
                interval_away_prob_est * 0.67..=interval_away_prob_est * 1.5,
                interval_common_prob_est * 0.67..=interval_common_prob_est * 1.5,
            ]
            .into(),
            resolution: 4,
        },
        |_| true,
        |values| {
            let mut biv_binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
            from_bivariate_binomial(
                INTERVALS as u8,
                values[0],
                values[1],
                values[2],
                &mut biv_binomial,
            );
            compute_mse(biv_poisson.flatten(), biv_binomial.flatten())
        },
    );
    println!("search_outcome: {search_outcome:?}");

    let mut biv_binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_bivariate_binomial(
        INTERVALS as u8,
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
        search_outcome.optimal_values[2],
        &mut biv_binomial,
    );
    println!(
        "biv_binomial:\n{}sum: {}",
        biv_binomial.verbose(),
        biv_binomial.flatten().sum()
    );

    let mse = compute_mse(biv_poisson.flatten(), biv_binomial.flatten());
    assert!(mse < 1e-3, "mse: {mse}");
}

#[test]
pub fn bivariate_binomial_interval_equivalence() {
    // 0.06208521833506868, 0.3083379160120557, 0.04249018964350848
    const INTERVAL_HOME_PROB: f64 = 0.06208521833506868;
    const INTERVAL_AWAY_PROB: f64 = 0.3083379160120557;
    const INTERVAL_COMMON_PROB: f64 = 0.04249018964350848;
    const INTERVALS: usize = 6;
    let mut biv_binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_bivariate_binomial(
        INTERVALS as u8,
        INTERVAL_HOME_PROB,
        INTERVAL_AWAY_PROB,
        INTERVAL_COMMON_PROB,
        &mut biv_binomial,
    );
    println!(
        "biv_binomial:\n{}sum: {}",
        biv_binomial.verbose(),
        biv_binomial.flatten().sum()
    );

    let mut interval = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_interval(
        INTERVALS as u8,
        0..INTERVALS as u8,
        u16::MAX,
        ScoringProbs {
            home_prob: INTERVAL_HOME_PROB,
            away_prob: INTERVAL_AWAY_PROB,
            common_prob: INTERVAL_COMMON_PROB,
        },
        ScoringProbs {
            home_prob: INTERVAL_HOME_PROB,
            away_prob: INTERVAL_AWAY_PROB,
            common_prob: INTERVAL_COMMON_PROB,
        },
        &mut interval,
    );
    println!(
        "interval:\n{}sum: {}",
        interval.verbose(),
        interval.flatten().sum()
    );

    let mse = compute_mse(biv_binomial.flatten(), interval.flatten());
    assert!(mse < 1e-9, "mse: {mse}");
}

fn compute_mse(sample_probs: &[f64], fitted_probs: &[f64]) -> f64 {
    let mut sq_error = 0.0;
    for (index, sample_prob) in sample_probs.iter().enumerate() {
        let fitted_prob: f64 = fitted_probs[index];
        sq_error += (sample_prob - fitted_prob).powi(2);
    }
    sq_error / sample_probs.len() as f64
}
