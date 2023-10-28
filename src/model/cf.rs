use serde::{Deserialize, Serialize};

use crate::data::Factor;
use crate::linear::regression::{Predictor, Regressor};

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