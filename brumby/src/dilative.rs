use crate::capture::Capture;
use crate::linear::matrix::Matrix;
use crate::probs::SliceExt;

#[derive(Default)]
pub struct DilatedProbs<'a> {
    win_probs: Option<Capture<'a, Vec<f64>, [f64]>>,
    dilatives: Option<Capture<'a, Vec<f64>, [f64]>>,
}
impl<'a> DilatedProbs<'a> {
    #[must_use]
    pub fn with_win_probs(mut self, win_probs: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.win_probs = Some(win_probs);
        self
    }

    #[must_use]
    pub fn with_dilatives(mut self, dilatives: Capture<'a, Vec<f64>, [f64]>) -> Self {
        self.dilatives = Some(dilatives);
        self
    }

    #[must_use]
    pub fn with_podium_places(self, podium_places: usize) -> Self {
        self.with_dilatives(Capture::Owned(vec![0.0; podium_places]))
    }
}

impl From<DilatedProbs<'_>> for Matrix<f64> {
    fn from(probs: DilatedProbs) -> Self {
        let win_probs = probs.win_probs.expect("no win probabilities specified");
        let dilatives = probs.dilatives.expect("no dilatives specified");
        let mut matrix = Matrix::allocate(dilatives.len(), win_probs.len());
        matrix.clone_row(&win_probs);
        dilatives.dilate_rows_power(&mut matrix);
        matrix
    }
}