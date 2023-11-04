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
    ZeroIntercept,
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
            Regressor::ZeroIntercept => 0.,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Regressor::Intercept | Regressor::ZeroIntercept)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RSquared {
    pub sum_sq_regression: f64,
    pub sum_sq_total: f64,
    pub independent_variables: usize,
    pub samples: usize,
}
impl RSquared {
    pub fn unadjusted(&self) -> f64 {
        1. - self.sum_sq_regression / self.sum_sq_total
    }

    pub fn adjusted(&self) -> f64 {
        1. - (1. - self.unadjusted())
            * ((self.samples - 1) as f64 / (self.samples - self.independent_variables - 1) as f64)
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
        let (mut sum_sq_regression, mut sum_sq_total) = (0., 0.);
        let mut sum = 0.;
        for row in data {
            let response = row[response_index];
            let predicted = self.predict(row);
            sum_sq_regression += (response - predicted).powi(2);
            sum += response;
        }
        let samples = data.rows();
        let mean = sum / samples as f64;
        for row in data {
            let response = row[response_index];
            sum_sq_total += (response - mean).powi(2);
        }
        let has_zero_intercept = self
            .regressors
            .iter()
            .any(|regressor| matches!(regressor, Regressor::ZeroIntercept));
        let zero_intercepts = if has_zero_intercept { 1 } else { 0 };
        // independent_variables: subtract 1 from number of regressors if intercept
        // present or 2 if no intercept
        RSquared {
            sum_sq_regression,
            sum_sq_total,
            independent_variables: (self.regressors.len() - 1 - zero_intercepts),
            samples,
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
            Regressor::<DummyOrdinal>::ZeroIntercept.to_string()
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
    pub r_squared: f64,
    pub r_squared_adj: f64,
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
        let r_squared = model.rsquared();
        let r_squared_adj = model.rsquared_adj();
        Ok(RegressionModel {
            response,
            predictor: Predictor {
                regressors,
                coefficients,
            },
            std_errors,
            p_values,
            r_squared,
            r_squared_adj,
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
                    format!("{:?}", regressor).into(),
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
mod tests {
    use assert_float_eq::*;
    use ordinalizer::Ordinal;

    use Regressor::{Exp, Ordinal, Product};

    use crate::linear::regression::Regressor::{Intercept, ZeroIntercept};
    use crate::testing::assert_slice_f64_relative;

    use super::*;

    #[derive(Debug, PartialEq, ordinalizer::Ordinal, Display, Serialize, Deserialize)]
    enum TestOrdinal {
        A,
        B,
    }

    impl AsIndex for TestOrdinal {
        fn as_index(&self) -> usize {
            self.ordinal()
        }
    }

    #[test]
    fn serde_json() {
        fn to_json(r: &Regressor<TestOrdinal>) -> String {
            serde_json::to_string(&r).unwrap()
        }

        fn from_json(json: &str) -> Regressor<TestOrdinal> {
            serde_json::from_str(&json).unwrap()
        }

        {
            let r = Ordinal(TestOrdinal::A);
            let json = to_json(&r);
            assert_eq!(r#"{"Ordinal":"A"}"#, json);
            let rr = from_json(&json);
            assert_eq!(r, rr);
        }
        {
            let r = Exp(Ordinal(TestOrdinal::A).into(), 5);
            let json = to_json(&r);
            assert_eq!(r#"{"Exp":[{"Ordinal":"A"},5]}"#, json);
            let rr = from_json(&json);
            assert_eq!(r, rr);
        }
        {
            let r = Product(vec![Ordinal(TestOrdinal::A), Ordinal(TestOrdinal::B)]);
            let json = to_json(&r);
            assert_eq!(r#"{"Product":[{"Ordinal":"A"},{"Ordinal":"B"}]}"#, json);
            let rr = from_json(&json);
            assert_eq!(r, rr);
        }
        {
            let r = Intercept;
            let json = to_json(&r);
            assert_eq!(r#""Intercept""#, json);
            let rr = from_json(&json);
            assert_eq!(r, rr);
        }
        {
            let r = ZeroIntercept;
            let json = to_json(&r);
            assert_eq!(r#""ZeroIntercept""#, json);
            let rr = from_json(&json);
            assert_eq!(r, rr);
        }
    }

    #[test]
    fn regression() {
        #[derive(Ordinal)]
        enum Factor {
            Y,
            X,
            W,
        }
        impl AsIndex for Factor {
            fn as_index(&self) -> usize {
                self.ordinal()
            }
        }

        #[rustfmt::skip]
        fn sample_data() -> Matrix<f64> {
            let mut data = Matrix::allocate(4, 3);
            data.flatten_mut()
                .clone_from_slice(&[
                    2., 2., 2.2,
                    3., 4., 1.8,
                    4., 6., 1.5,
                    6., 7., 1.1
                ]);
            data
        }
        let data = sample_data();
        const EPSILON: f64 = 1e-13;
        {
            // with intercept
            let model = RegressionModel::fit(Factor::Y, vec![Intercept, Ordinal(Factor::X)], &data)
                .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[0.28813559322033333, 0.7288135593220351],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.std_errors,
                &[0.9024528482694316, 0.1761407600917501],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.p_values,
                &[0.7797772260959455, 0.05374447650832757],
                EPSILON,
            );
            assert_float_relative_eq!(0.895399515738499, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.8430992736077485, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // without intercept
            let model =
                RegressionModel::fit(Factor::Y, vec![ZeroIntercept, Ordinal(Factor::X)], &data)
                    .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[0.0, 0.7809523809523811],
                EPSILON,
            );
            assert_slice_f64_relative(&model.std_errors, &[0.0, 0.05525998471596577], EPSILON);
            assert_slice_f64_relative(&model.p_values, &[1.0, 0.0007674606469419348], EPSILON);
            assert_float_relative_eq!(0.8900680272108843, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.8900680272108843, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // with square term
            let model = RegressionModel::fit(
                Factor::Y,
                vec![
                    Intercept,
                    Ordinal(Factor::X),
                    Exp(Ordinal(Factor::X).into(), 2),
                ],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[2.6281407035175928, -0.5552763819095485, 0.14321608040201017],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.std_errors,
                &[2.0551227965369234, 1.0499867426753304, 0.11579616705329593],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.p_values,
                &[0.422492016413063, 0.6903140400788879, 0.43285536174646816],
                EPSILON,
            );
            assert_float_relative_eq!(0.9586503948312993, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.875951184493898, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // with multiple distinct regressors
            let model = RegressionModel::fit(
                Factor::Y,
                vec![Intercept, Ordinal(Factor::X), Ordinal(Factor::W)],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[17.60526315789471, -0.631578947368419, -6.578947368421037],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.std_errors,
                &[5.333802206998271, 0.4243293551736085, 2.0213541441759535],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.p_values,
                &[0.18727824790649023, 0.37661521814453486, 0.1897706353451349],
                EPSILON,
            );
            assert_float_relative_eq!(0.9909774436090225, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.9729323308270675, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // with multiple distinct regressors
            let model = RegressionModel::fit(
                Factor::Y,
                vec![Intercept, Ordinal(Factor::X), Ordinal(Factor::W)],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[17.60526315789471, -0.631578947368419, -6.578947368421037],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.std_errors,
                &[5.333802206998271, 0.4243293551736085, 2.0213541441759535],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.p_values,
                &[0.18727824790649023, 0.37661521814453486, 0.1897706353451349],
                EPSILON,
            );
            assert_float_relative_eq!(0.9909774436090225, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.9729323308270675, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // with multiple distinct regressors and no intercept
            let model = RegressionModel::fit(
                Factor::Y,
                vec![ZeroIntercept, Ordinal(Factor::X), Ordinal(Factor::W)],
                &data,
            )
                .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[0.0, 0.760351500693751, 0.0764343613836096],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.std_errors,
                &[0.0,  0.11484474436354505, 0.34641993071948335],
                EPSILON,
            );
            assert_slice_f64_relative(
                &model.p_values,
                &[1.0, 0.022061436034720366, 0.8458482505584344],
                EPSILON,
            );
            assert_float_relative_eq!(0.8926803145006207, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.8390204717509311, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
        {
            // with interaction term and zero intercept
            let model = RegressionModel::fit(
                Factor::Y,
                vec![
                    ZeroIntercept,
                    Product(vec![Ordinal(Factor::X), Ordinal(Factor::W)]),
                ],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.predictor.coefficients,
                &[0.0, 0.5324128800416095],
                EPSILON,
            );
            assert_slice_f64_relative(&model.std_errors, &[0.0, 0.08921820060416732], EPSILON);
            assert_slice_f64_relative(&model.p_values, &[1.0, 0.00941534405942245], EPSILON);
            assert_float_relative_eq!(0.42282174773545533, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.42282174773545533, model.r_squared_adj, EPSILON);
            assert_float_relative_eq!(
                model.r_squared,
                model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
                EPSILON
            );
            assert_float_relative_eq!(
                model.r_squared_adj,
                model.predictor.r_squared(&Factor::Y, &data).adjusted(),
                EPSILON
            );
        }
    }

    #[test]
    fn significance_resolve() {
        assert_eq!(Significance::A, Significance::lookup(0.0));
        assert_eq!(Significance::A, Significance::lookup(0.0009));
        assert_eq!(Significance::B, Significance::lookup(0.001));
        assert_eq!(Significance::B, Significance::lookup(0.009));
        assert_eq!(Significance::C, Significance::lookup(0.01));
        assert_eq!(Significance::C, Significance::lookup(0.049));
        assert_eq!(Significance::D, Significance::lookup(0.05));
        assert_eq!(Significance::D, Significance::lookup(0.09));
        assert_eq!(Significance::E, Significance::lookup(0.1));
        assert_eq!(Significance::E, Significance::lookup(1.0));
    }
}
