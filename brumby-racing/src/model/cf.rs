use ordinalizer::Ordinal;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumCount, EnumIter};
use brumby::linear::regression;
use brumby::linear::regression::{AsIndex, Predictor, Regressor};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coefficients {
    pub w1: Predictor<Factor>,
    pub w2: Predictor<Factor>,
    pub w3: Predictor<Factor>,
}
impl Coefficients {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        self.w1.validate()?;
        self.w2.validate()?;
        self.w3.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regressors {
    pub w1: Vec<Regressor<Factor>>,
    pub w2: Vec<Regressor<Factor>>,
    pub w3: Vec<Regressor<Factor>>,
}
impl Regressors {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        regression::validate_regressors(&self.w1)?;
        regression::validate_regressors(&self.w2)?;
        regression::validate_regressors(&self.w3)?;
        Ok(())
    }
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