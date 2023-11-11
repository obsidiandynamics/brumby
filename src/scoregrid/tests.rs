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
    populate_matrix(iter, &mut matrix);
    println!("matrix:\n{}", matrix.verbose());
    println!("sum: {}", matrix.flatten().sum());
}