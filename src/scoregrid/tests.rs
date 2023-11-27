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