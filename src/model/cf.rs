use ordinalizer::Ordinal;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumCount, EnumIter};

use crate::linear::regression::{AsIndex, Predictor, Regressor};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coefficients {
    pub w1: Predictor<Factor>,
    pub w2: Predictor<Factor>,
    pub w3: Predictor<Factor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regressors {
    pub w1: Vec<Regressor<Factor>>,
    pub w2: Vec<Regressor<Factor>>,
    pub w3: Vec<Regressor<Factor>>,
}

#[derive(Debug, Clone, PartialEq, Ordinal, EnumCount, EnumIter, Display, Serialize, Deserialize)]
pub enum Factor {
    RaceId,
    RunnerIndex,
    ActiveRunners,
    PlacesPaying,
    Stdev,
    Weight0,
    Weight1,
    Weight2,
    Weight3,
}

impl From<Factor> for usize {
    fn from(factor: Factor) -> Self {
        factor.ordinal()
    }
}

impl AsIndex for Factor {
    fn as_index(&self) -> usize {
        self.ordinal()
    }
}