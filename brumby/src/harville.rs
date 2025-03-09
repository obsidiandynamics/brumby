use crate::comb::{count_permutations, is_unique_linear, pick};
use crate::linear::matrix::Matrix;

pub fn harville(probs: &Matrix<f64>, podium: &[usize]) -> f64 {
    let mut combined_prob = 1.;
    // println!("probs: {probs:?}, podium: {podium:?}");
    for (rank, rank_probs) in probs.into_iter().enumerate() {
        let runner = podium[rank];
        let mut remaining_prob = 1.;
        for prev_rank in 0..rank {
            remaining_prob -= rank_probs[podium[prev_rank]];
        }
        let prob = rank_probs[runner];
        combined_prob *= prob / remaining_prob;
        // println!("  rank: {rank}, prob: {prob}, combined_prob: {combined_prob}, remaining_prob: {remaining_prob}");
    }
    combined_prob
}

pub fn harville_summary(probs: &Matrix<f64>, ranks: usize) -> Matrix<f64> {
    let runners = probs.cols();
    let mut summary = Matrix::allocate(ranks, runners);
    let cardinalities = vec![runners; ranks];
    let mut podium = vec![0; ranks];
    let mut bitmap = vec![false; runners];
    harville_summary_no_alloc(
        probs,
        ranks,
        &cardinalities,
        &mut podium,
        &mut bitmap,
        &mut summary,
    );
    summary
}

pub fn harville_summary_no_alloc(
    probs: &Matrix<f64>,
    ranks: usize,
    cardinalities: &[usize],
    podium: &mut [usize],
    bitmap: &mut [bool],
    summary: &mut Matrix<f64>,
) {
    debug_assert_eq!(
        probs.rows(),
        ranks,
        "number of rows in the probabilities matrix must equal to the number of ranks"
    );
    debug_assert_eq!(summary.rows(), probs.rows(), "number of rows in the probabilities matrix must equal to the number of rows in the summary matrix");
    debug_assert_eq!(summary.cols(), probs.cols(), "number of columns in the probabilities matrix must equal to the number of columns in the summary matrix");
    debug_assert_eq!(
        probs.rows(),
        podium.len(),
        "number of rows in the probabilities matrix must equal to the podium length"
    );
    debug_assert_eq!(
        probs.cols(),
        bitmap.len(),
        "number of columns in the probabilities matrix must equal to the bitmap length"
    );
    let permutations = count_permutations(cardinalities);
    for permutation in 0..permutations {
        pick(cardinalities, permutation, podium);
        if !is_unique_linear(podium, bitmap) {
            continue;
        }
        let prob = harville(probs, podium);
        for (rank, &runner) in podium.iter().enumerate() {
            summary[(rank, runner)] += prob;
        }
    }
}

pub fn harville_summary_condensed(probs: &Matrix<f64>, ranks: usize) -> Vec<f64> {
    let runners = probs.cols();
    let mut summary = Vec::with_capacity(runners);
    summary.resize(runners, 0.0);
    let cardinalities = vec![runners; ranks];
    let mut podium = vec![0; ranks];
    let mut bitmap = vec![false; runners];
    harville_summary_condensed_no_alloc(
        probs,
        ranks,
        &cardinalities,
        &mut podium,
        &mut bitmap,
        summary.as_mut_slice(),
    );
    summary
}

pub fn harville_summary_condensed_no_alloc(
    probs: &Matrix<f64>,
    ranks: usize,
    cardinalities: &[usize],
    podium: &mut [usize],
    bitmap: &mut [bool],
    summary: &mut [f64],
) {
    debug_assert_eq!(
        probs.rows(),
        ranks,
        "number of rows in the probabilities matrix must equal to the number of ranks"
    );
    debug_assert_eq!(summary.len(), probs.cols(), "number of columns in the probabilities matrix must equal to the length of the summary slice");
    debug_assert_eq!(
        probs.rows(),
        podium.len(),
        "number of rows in the probabilities matrix must equal to the podium length"
    );
    debug_assert_eq!(
        probs.cols(),
        bitmap.len(),
        "number of columns in the probabilities matrix must equal to the bitmap length"
    );
    let permutations = count_permutations(cardinalities);
    for permutation in 0..permutations {
        pick(cardinalities, permutation, podium);
        if !is_unique_linear(podium, bitmap) {
            continue;
        }
        let prob = harville(probs, podium);
        for &runner in podium.iter() {
            summary[runner] += prob;
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::assert_float_relative_eq;
    use assert_float_eq::*;
    use brumby_testing::assert_slice_f64_relative;

    use crate::capture::Capture;
    use crate::comb::{is_unique_quadratic, Permuter};
    use crate::dilative::DilatedProbs;
    use crate::probs::SliceExt;

    use super::*;

    #[derive(Debug)]
    struct PodiumProb {
        podium: Vec<usize>,
        prob: f64,
    }

    #[test]
    fn harville_3x3_without_scratchings() {
        const WIN_PROBS: [f64; 3] = [0.6, 0.3, 0.1];
        const RANKS: usize = 3;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(3),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(6, probs.len());

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_3x4_with_scratching() {
        const WIN_PROBS: [f64; 4] = [0.6, 0.3, 0.1, 0.0];
        const RANKS: usize = 3;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(24, probs.len());

        let nonzero_scratched = probs
            .iter()
            .find(|&podium_prob| podium_prob.podium.contains(&3) && podium_prob.prob != 0.0);
        assert!(nonzero_scratched.is_none());

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_4x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const RANKS: usize = 4;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(24, probs.len());
        println!("probs: {probs:?}");

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_1x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.6, 0.3, 0.1, 0.0];
        const RANKS: usize = 1;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(4, probs.len());

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_2x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.6, 0.3, 0.1, 0.0];
        const RANKS: usize = 2;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(12, probs.len());

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_4x4_without_scratchings_dilated() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const DILATIVES: [f64; 4] = [0.0, 0.1, 0.2, 0.3];
        const RANKS: usize = 4;
        const RUNNERS: usize = WIN_PROBS.len();
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_dilatives(Capture::Borrowed(&DILATIVES)),
        );
        let permuter = Permuter::new(&[RUNNERS; RANKS]);
        let probs = permuter
            .into_iter()
            .filter(|podium| is_unique_quadratic(&podium))
            .map(|podium| {
                let prob = harville(&probs, &podium);
                PodiumProb { podium, prob }
            })
            .collect::<Vec<_>>();
        assert_eq!(24, probs.len());

        let sum = probs
            .iter()
            .map(|podium_prob| podium_prob.prob)
            .sum::<f64>();
        assert_float_relative_eq!(1.0, sum);
    }

    #[test]
    fn harville_summary_3x3_without_scratchings() {
        const WIN_PROBS: [f64; 3] = [0.6, 0.3, 0.1];
        const RANKS: usize = 3;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary(&probs, RANKS);
        println!("summary:\n{}", summary.verbose());
        assert_slice_f64_relative(
            &[
                0.6,
                0.3,
                0.1,
                0.32380952380952444,
                0.48333333333333445,
                0.19285714285714314,
                0.07619047619047627,
                0.216666666666667,
                0.7071428571428587,
            ],
            summary.flatten(),
            1e-9,
        );

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            assert_float_relative_eq!(1.0, col_cells.sum::<f64>());
        }
    }

    #[test]
    fn harville_summary_3x2_condensed_without_scratchings() {
        const WIN_PROBS: [f64; 3] = [0.6, 0.3, 0.1];
        const RANKS: usize = 2;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary_condensed(&probs, RANKS);
        println!("summary: {summary:?}");
        assert_slice_f64_relative(
            &[
                0.6 + 0.32380952380952444,
                0.3 + 0.48333333333333445,
                0.1 + 0.19285714285714314,
            ],
            &summary,
            1e-9,
        );
    }

    #[test]
    fn harville_summary_condensed_3x3_without_scratchings() {
        const WIN_PROBS: [f64; 3] = [0.6, 0.3, 0.1];
        const RANKS: usize = 3;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary_condensed(&probs, RANKS);
        println!("summary: {summary:?}");
        assert_slice_f64_relative(
            &[
                1.0,
                1.0,
                1.0,
            ],
            &summary,
            1e-9,
        );
    }

    #[test]
    fn harville_summary_3x4_with_scratching() {
        const WIN_PROBS: [f64; 4] = [0.6, 0.3, 0.1, 0.0];
        const RANKS: usize = 3;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary(&probs, RANKS);
        println!("summary:\n{}", summary.verbose());
        assert_slice_f64_relative(
            &[
                0.6,
                0.3,
                0.1,
                0.0,
                0.32380952380952444,
                0.48333333333333445,
                0.19285714285714314,
                0.0,
                0.07619047619047627,
                0.216666666666667,
                0.7071428571428587,
                0.0,
            ],
            summary.flatten(),
            1e-9,
        );

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            if col == 3 {
                assert_float_relative_eq!(0.0, col_cells.sum::<f64>());
            } else {
                assert_float_relative_eq!(1.0, col_cells.sum::<f64>());
            }
        }
    }

    #[test]
    fn harville_summary_4x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const RANKS: usize = 4;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary(&probs, RANKS);
        assert_eq!(RANKS, summary.rows());
        assert_eq!(WIN_PROBS.len(), summary.cols());
        assert_slice_f64_relative(&WIN_PROBS, &summary[0], 1e-9);
        println!("summary:\n{}", summary.verbose());

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            assert_float_relative_eq!(1.0, col_cells.sum::<f64>());
        }
    }

    #[test]
    fn harville_summary_1x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const RANKS: usize = 1;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary(&probs, RANKS);
        assert_eq!(RANKS, summary.rows());
        assert_eq!(WIN_PROBS.len(), summary.cols());
        assert_slice_f64_relative(&WIN_PROBS, &summary[0], 1e-9);
        println!("summary:\n{}", summary.verbose());

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            assert!(col_cells.sum::<f64>() <= 1.0);
        }
    }

    #[test]
    fn harville_summary_2x4_without_scratchings() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const RANKS: usize = 2;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_podium_places(RANKS),
        );
        let summary = harville_summary(&probs, RANKS);
        assert_eq!(RANKS, summary.rows());
        assert_eq!(WIN_PROBS.len(), summary.cols());
        assert_slice_f64_relative(&WIN_PROBS, &summary[0], 1e-9);
        println!("summary:\n{}", summary.verbose());

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            assert!(col_cells.sum::<f64>() <= 1.0);
        }
    }

    #[test]
    fn harville_summary_4x4_without_scratchings_dilated() {
        const WIN_PROBS: [f64; 4] = [0.4, 0.3, 0.2, 0.1];
        const DILATIVES: [f64; 4] = [0.0, 0.1, 0.2, 0.3];
        const RANKS: usize = 4;
        let probs = Matrix::from(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&WIN_PROBS))
                .with_dilatives(Capture::Borrowed(&DILATIVES)),
        );
        let summary = harville_summary(&probs, RANKS);
        assert_eq!(RANKS, summary.rows());
        assert_eq!(WIN_PROBS.len(), summary.cols());
        assert_slice_f64_relative(&WIN_PROBS, &summary[0], 1e-9);
        println!("summary:\n{}", summary.verbose());

        for row in summary.into_iter() {
            assert_float_relative_eq!(1.0, row.sum());
        }
        for col in 0..summary.cols() {
            let col_cells = summary.col(col);
            assert_float_relative_eq!(1.0, col_cells.sum::<f64>());
        }
    }
}
