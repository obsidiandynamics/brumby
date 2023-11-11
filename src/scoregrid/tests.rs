use crate::opt::{hypergrid_search, HypergridSearchConfig};
use crate::probs::SliceExt;
use super::*;

#[test]
pub fn iterate_scoregrid_5x5() {
    const INTERVALS: usize = 4;
    let space = ScoreOutcomeSpace {
        interval_home_prob: 0.25,
        interval_away_prob: 0.2,
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
    assert_eq!(0.65, Outcome::Win(Side::Home).gather(&scoregrid));
    assert_eq!(0.15, Outcome::Win(Side::Away).gather(&scoregrid));
}

#[test]
pub fn outcome_draw_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.2, Outcome::Draw.gather(&scoregrid));
}

#[test]
pub fn outcome_goals_ou_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.35, Outcome::GoalsUnder(3).gather(&scoregrid));
    assert_eq!(0.65, Outcome::GoalsOver(2).gather(&scoregrid));
}

#[test]
pub fn outcome_correct_score_gather() {
    let scoregrid = create_test_4x4_scoregrid();
    assert_eq!(0.04, Outcome::CorrectScore(Score::new(0, 0)).gather(&scoregrid));
    assert_eq!(0.08, Outcome::CorrectScore(Score::new(3, 2)).gather(&scoregrid));
}

#[test]
pub fn univariate_poisson_binomial_similarity() {
    const HOME_RATE: f64 = 1.2;
    const AWAY_RATE: f64 = 1.8;
    const INTERVALS: usize = 6;
    let mut poisson = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_univariate_poisson(HOME_RATE, AWAY_RATE, &mut poisson);
    println!("poisson:\n{}sum: {}", poisson.verbose(), poisson.flatten().sum());

    let interval_home_prob_est = 1.0 - poisson::univariate(0, HOME_RATE / INTERVALS as f64, &factorial::Calculator);
    let interval_away_prob_est = 1.0 - poisson::univariate(0, AWAY_RATE / INTERVALS as f64, &factorial::Calculator);
    println!("estimated home_prob: {interval_home_prob_est}, away_prob: {interval_away_prob_est}");
    let search_outcome = hypergrid_search(&HypergridSearchConfig {
        max_steps: 10,
        acceptable_residual: 1e-6,
        bounds: vec![interval_home_prob_est * 0.67..=interval_home_prob_est * 1.5, interval_away_prob_est * 0.67..=interval_away_prob_est * 1.5].into(),
        resolution: 4,
    }, |values| {
        let mut binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
        from_binomial(values[0], values[1], &mut binomial);
        compute_mse(poisson.flatten(), binomial.flatten())
    });
    println!("search_outcome: {search_outcome:?}");

    let mut binomial = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    from_binomial(search_outcome.optimal_values[0], search_outcome.optimal_values[1], &mut binomial);
    println!("binomial:\n{}sum: {}", binomial.verbose(), binomial.flatten().sum());

    let mse = compute_mse(poisson.flatten(), binomial.flatten());
    assert!(mse < 1e-3, "mse: {mse}");
}

fn compute_mse(sample_probs: &[f64], fitted_probs: &[f64]) -> f64 {
    let mut sq_error = 0.0;
    for (index, sample_prob) in sample_probs.iter().enumerate() {
        let fitted_prob: f64 = fitted_probs[index];
        sq_error += (sample_prob - fitted_prob).powi(2);
    }
    sq_error / sample_probs.len() as f64
}
// #[test]
// pub fn wierd() {
//     let home_rate = 0.7;
//     let away_rate = 0.8;
//     let common = 0.2;
//     let mut biv_poisson = Matrix::allocate(4, 4);
//     from_bivariate_poisson(home_rate, away_rate, common, &mut biv_poisson);
//     println!("biv_poisson:\n{}sum: {}", biv_poisson.verbose(), biv_poisson.flatten().sum());
//
//     let mut weird = Matrix::allocate(4, 4);
//     from_wierd(home_rate, away_rate, common, &mut weird);
//     println!("biv_poisson:\n{}sum: {}", weird.verbose(), weird.flatten().sum());
// }