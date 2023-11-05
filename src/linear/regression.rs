//! Linear regression.

use core::fmt::Debug;
use std::ops::Range;
use std::string::ToString;

use anyhow::bail;
use linregress::fit_low_level_regression_model;
use serde::{Deserialize, Serialize};
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumCount, EnumIter};

use crate::linear::matrix::Matrix;

pub trait AsIndex {
    fn as_index(&self) -> usize;
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Display)]
pub enum Regressor<O: AsIndex> {
    Ordinal(O),
    Exp(Box<Regressor<O>>, i32),
    Product(Vec<Regressor<O>>),
    Intercept,
    Origin,
}
impl<O: AsIndex> Regressor<O> {
    pub fn resolve(&self, input: &[f64]) -> f64 {
        match self {
            Regressor::Ordinal(ordinal) => input[ordinal.as_index()],
            Regressor::Exp(regressor, power) => regressor.resolve(input).powi(*power),
            Regressor::Product(regressors) => regressors
                .iter()
                .map(|regressor| regressor.resolve(input))
                .product(),
            Regressor::Intercept => 1.,
            Regressor::Origin => 0.,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Regressor::Intercept | Regressor::Origin)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RSquared {
    pub sum_sq_residual: f64,
    pub sum_sq_total: f64,
    pub df_residual: usize,
    pub df_total: usize,
}
impl RSquared {
    pub fn unadjusted(&self) -> f64 {
        1. - self.sum_sq_residual / self.sum_sq_total
    }

    pub fn adjusted(&self) -> f64 {
        1. - self.sum_sq_residual / self.sum_sq_total * self.df_total as f64 / self.df_residual as f64
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Predictor<O: AsIndex> {
    pub regressors: Vec<Regressor<O>>,
    pub coefficients: Vec<f64>,
}
impl<O: AsIndex> Predictor<O> {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        validate_regressors(&self.regressors)?;
        if self.regressors.len() != self.coefficients.len() {
            bail!("exactly one coefficient must be specified for each regressor");
        }
        Ok(())
    }

    pub fn predict(&self, input: &[f64]) -> f64 {
        self.regressors
            .iter()
            .enumerate()
            .map(|(regressor_index, regressor)| {
                let coefficient = self.coefficients[regressor_index];
                coefficient * regressor.resolve(input)
            })
            .sum()
    }

    pub fn r_squared(&self, response: &O, data: &Matrix<f64>) -> RSquared {
        let response_index = response.as_index();
        let (mut sum_sq_residual, mut sum_sq_total) = (0., 0.);
        let mut sum = 0.;
        for row in data {
            let response = row[response_index];
            let predicted = self.predict(row);
            sum_sq_residual += (response - predicted).powi(2);
            sum += response;
        }
        let samples = data.rows();

        let has_zero_intercept = self
            .regressors
            .iter()
            .any(|regressor| matches!(regressor, Regressor::Origin));

        let df_residual;
        let df_total;
        if has_zero_intercept {
            // emulates the behaviour of R for deriving both unadjusted and adjusted r-squared
            // when the intercept is suppressed
            for row in data {
                let response = row[response_index];
                sum_sq_total += response.powi(2);
            }
            df_residual = samples - self.regressors.len() + 1;
            df_total = samples;
        } else {
            // standard derivation of unadjusted and adjusted r-squared when an intercept term
            // is present
            let mean = sum / samples as f64;
            for row in data {
                let response = row[response_index];
                sum_sq_total += (response - mean).powi(2);
            }
            df_residual = samples - self.regressors.len();
            df_total = samples - 1;
        }

        RSquared {
            sum_sq_residual,
            sum_sq_total,
            df_residual,
            df_total
        }
    }
}

pub(crate) fn validate_regressors<O: AsIndex>(
    regressors: &[Regressor<O>],
) -> Result<(), anyhow::Error> {
    if regressors.len() < 2 {
        bail!("at least two regressors must be present");
    }
    let constants = regressors
        .iter()
        .filter(|regressor| regressor.is_constant())
        .count();
    if constants != 1 {
        bail!(
            "must specify exactly one {} or {} regressor",
            Regressor::<DummyOrdinal>::Intercept.to_string(),
            Regressor::<DummyOrdinal>::Origin.to_string()
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegressionModel<O: AsIndex> {
    pub response: O,
    pub predictor: Predictor<O>,
    pub std_errors: Vec<f64>,
    pub p_values: Vec<f64>,
    pub r_squared: RSquared,
}
impl<O: AsIndex> RegressionModel<O> {
    pub fn fit(
        response: O,
        regressors: Vec<Regressor<O>>,
        data: &Matrix<f64>,
    ) -> Result<Self, anyhow::Error> {
        if data.cols() < 2 {
            bail!("insufficient number of columns in the data");
        }
        validate_regressors(&regressors)?;

        let mut subset: Matrix<f64> = Matrix::allocate(data.rows(), 1 + regressors.len());
        for (row_index, row_data) in data.into_iter().enumerate() {
            subset[(row_index, 0)] = row_data[response.as_index()];
            for (regressor_index, regressor) in regressors.iter().enumerate() {
                subset[(row_index, 1 + regressor_index)] = regressor.resolve(row_data);
            }
        }

        let model = fit_low_level_regression_model(subset.flatten(), subset.rows(), subset.cols())?;
        let coefficients = model.parameters().to_vec();
        let std_errors = model.se().to_vec();
        let p_values = model.p_values().to_vec();
        let predictor = Predictor {
            regressors,
            coefficients,
        };
        let r_squared = predictor.r_squared(&response, data);
        Ok(RegressionModel {
            response,
            predictor,
            std_errors,
            p_values,
            r_squared
        })
    }

    pub fn tabulate(&self) -> Table
    where
        O: Debug,
    {
        let mut table = Table::default()
            .with_cols(vec![
                Col::new(Styles::default()),
                Col::new(Styles::default().with(MinWidth(12)).with(HAlign::Right)),
                Col::new(Styles::default().with(MinWidth(11)).with(HAlign::Right)),
                Col::new(Styles::default().with(MinWidth(9)).with(HAlign::Right)),
                Col::new(Styles::default().with(MinWidth(5))),
            ])
            .with_row(Row::new(
                Styles::default().with(Header(true)),
                vec![
                    "Regressor".into(),
                    "Coefficient".into(),
                    "Std. error".into(),
                    "P-value".into(),
                    "".into(),
                ],
            ));
        for (regressor_index, regressor) in self.predictor.regressors.iter().enumerate() {
            table.push_row(Row::new(
                Styles::default(),
                vec![
                    format!("{regressor:?}").into(),
                    format!("{:.8}", self.predictor.coefficients[regressor_index]).into(),
                    format!("{:.6}", self.std_errors[regressor_index]).into(),
                    format!("{:.6}", self.p_values[regressor_index]).into(),
                    Significance::lookup(self.p_values[regressor_index])
                        .to_string()
                        .into(),
                ],
            ));
        }

        table
    }
}

#[derive(Debug, Clone, PartialEq, EnumCount, EnumIter)]
pub enum Significance {
    A,
    B,
    C,
    D,
    E,
}
impl Significance {
    pub fn label(&self) -> &'static str {
        match self {
            Significance::A => "***",
            Significance::B => "**",
            Significance::C => "*",
            Significance::D => ".",
            Significance::E => "",
        }
    }

    pub fn range(&self) -> Range<f64> {
        match self {
            Significance::A => 0.0..0.001,
            Significance::B => 0.001..0.01,
            Significance::C => 0.01..0.05,
            Significance::D => 0.05..0.1,
            Significance::E => 0.1..1.0 + f64::EPSILON,
        }
    }

    pub fn lookup(p_value: f64) -> Self {
        for sig in Self::iter() {
            if sig.range().contains(&p_value) {
                return sig;
            }
        }
        unreachable!()
    }
}
impl ToString for Significance {
    fn to_string(&self) -> String {
        self.label().into()
    }
}

struct DummyOrdinal;
impl AsIndex for DummyOrdinal {
    fn as_index(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests;
