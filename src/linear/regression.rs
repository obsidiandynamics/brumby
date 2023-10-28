//! Linear regression.

use linregress::fit_low_level_regression_model;
use serde::{Deserialize, Serialize};

use crate::linear::matrix::Matrix;

pub trait AsIndex {
    fn as_index(&self) -> usize;
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Regressor<O: AsIndex> {
    Ordinal(O),
    Exponent(Box<Regressor<O>>, i32),
    Product(Vec<Regressor<O>>),
    Intercept,
    NilIntercept,
}

impl<O: AsIndex> Regressor<O> {
    pub fn resolve(&self, input: &[f64]) -> f64 {
        match self {
            Regressor::Ordinal(ordinal) => input[ordinal.as_index()],
            Regressor::Exponent(regressor, power) => regressor.resolve(input).powi(*power),
            Regressor::Product(regressors) => regressors
                .iter()
                .map(|regressor| regressor.resolve(input))
                .product(),
            Regressor::Intercept => 1.,
            Regressor::NilIntercept => 0.,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RegressionModel<O: AsIndex> {
    pub response: O,
    pub regressors: Vec<Regressor<O>>,
    pub coefficients: Vec<f64>,
    pub std_errors: Vec<f64>,
    pub p_values: Vec<f64>,
    pub r_squared: f64,
    pub r_squared_adj: f64,
}

pub fn fit<O: AsIndex>(
    response: O,
    regressors: Vec<Regressor<O>>,
    data: &Matrix<f64>,
) -> Result<RegressionModel<O>, anyhow::Error> {
    let mut subset: Matrix<f64> = Matrix::allocate(data.rows(), 1 + regressors.len());
    for (row_index, row_data) in data.into_iter().enumerate() {
        subset[(row_index, 0)] = row_data[response.as_index()];
        for (regressor_index, regressor) in regressors.iter().enumerate() {
            subset[(row_index, 1 + regressor_index)] = regressor.resolve(row_data);
        }
    }

    // println!("subset: \n{}", subset.verbose());
    let model = fit_low_level_regression_model(subset.flatten(), subset.rows(), subset.cols())?;
    // println!("params: {:?}", model.parameters());
    // println!("std_errors: {:?}", model.se());
    // println!("p_values: {:?}", model.p_values());
    // println!("r_squared: {}", model.rsquared());
    // println!("r_squared_adj: {}", model.rsquared_adj());

    let coefficients = model.parameters().iter().map(|&val| val).collect();
    let std_errors = model.se().iter().map(|&val| val).collect();
    let p_values = model.p_values().iter().map(|&val| val).collect();
    let r_squared = model.rsquared();
    let r_squared_adj = model.rsquared_adj();
    Ok(RegressionModel {
        response,
        regressors,
        coefficients,
        std_errors,
        p_values,
        r_squared,
        r_squared_adj,
    })
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use ordinalizer::Ordinal;

    use Regressor::{Exponent, Ordinal, Product};

    use crate::linear::regression::Regressor::{Intercept, NilIntercept};
    use crate::testing::{assert_slice_f64_relative};

    use super::*;

    #[derive(
        Debug, PartialEq, ordinalizer::Ordinal, strum_macros::Display, Serialize, Deserialize,
    )]
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
            let r = Exponent(Box::new(Ordinal(TestOrdinal::A)), 5);
            let json = to_json(&r);
            assert_eq!(r#"{"Exponent":[{"Ordinal":"A"},5]}"#, json);
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
            let r = NilIntercept;
            let json = to_json(&r);
            assert_eq!(r#""NilIntercept""#, json);
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
        const EPSILON: f64 = 1e-15;
        {
            // with intercept
            let model = fit(Factor::Y, vec![Intercept, Ordinal(Factor::X)], &data).unwrap();
            assert_slice_f64_relative(
                &model.coefficients,
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
        }
        {
            // without intercept
            let model = fit(Factor::Y, vec![NilIntercept, Ordinal(Factor::X)], &data).unwrap();
            assert_slice_f64_relative(&model.coefficients, &[0.0, 0.7809523809523811], EPSILON);
            assert_slice_f64_relative(&model.std_errors, &[0.0, 0.05525998471596577], EPSILON);
            assert_slice_f64_relative(&model.p_values, &[1.0, 0.0007674606469419348], EPSILON);
            assert_float_relative_eq!(0.8900680272108843, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.8900680272108843, model.r_squared_adj, EPSILON);
        }
        {
            // with square term
            let model = fit(
                Factor::Y,
                vec![
                    Intercept,
                    Ordinal(Factor::X),
                    Exponent(Box::new(Ordinal(Factor::X)), 2),
                ],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.coefficients,
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
        }
        {
            // with multiple distinct regressors
            let model = fit(
                Factor::Y,
                vec![Intercept, Ordinal(Factor::X), Ordinal(Factor::W)],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(
                &model.coefficients,
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
        }
        {
            // with interaction term and zero intercept
            let model = fit(
                Factor::Y,
                vec![
                    NilIntercept,
                    Product(vec![Ordinal(Factor::X), Ordinal(Factor::W)]),
                ],
                &data,
            )
            .unwrap();
            assert_slice_f64_relative(&model.coefficients, &[0.0, 0.5324128800416095], EPSILON);
            assert_slice_f64_relative(&model.std_errors, &[0.0, 0.08921820060416732], EPSILON);
            assert_slice_f64_relative(&model.p_values, &[1.0, 0.00941534405942245], EPSILON);
            assert_float_relative_eq!(0.42282174773545533, model.r_squared, EPSILON);
            assert_float_relative_eq!(0.42282174773545533, model.r_squared_adj, EPSILON);
        }
    }
}
