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
        let model =
            RegressionModel::fit(Factor::Y, vec![Intercept, Ordinal(Factor::X)], &data).unwrap();
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
        assert_float_relative_eq!(0.895399515738499, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.8430992736077485, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
            model.predictor.r_squared(&Factor::Y, &data).adjusted(),
            EPSILON
        );
    }
    {
        // without intercept
        let model = RegressionModel::fit(Factor::Y, vec![ZeroIntercept, Ordinal(Factor::X)], &data)
            .unwrap();
        assert_slice_f64_relative(
            &model.predictor.coefficients,
            &[0.0, 0.7809523809523811],
            EPSILON,
        );
        assert_slice_f64_relative(&model.std_errors, &[0.0, 0.05525998471596577], EPSILON);
        assert_slice_f64_relative(&model.p_values, &[1.0, 0.0007674606469419348], EPSILON);
        assert_float_relative_eq!(0.9852014652014652, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.9802686202686203, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
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
        assert_float_relative_eq!(0.9586503948312993, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.875951184493898, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
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
        assert_float_relative_eq!(0.9909774436090225, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.9729323308270675, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
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
            &[0.0, 0.11484474436354505, 0.34641993071948335],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.p_values,
            &[1.0, 0.022061436034720366, 0.8458482505584344],
            EPSILON,
        );
        assert_float_relative_eq!(0.9855531192596989, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.9711062385193979, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
            model.predictor.r_squared(&Factor::Y, &data).adjusted(),
            EPSILON
        );
    }
    {
        // with interaction term and no intercept
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
        assert_float_relative_eq!(0.9223029275797728, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.8964039034396971, model.r_squared.adjusted(), EPSILON);
        assert_float_relative_eq!(
            model.r_squared.unadjusted(),
            model.predictor.r_squared(&Factor::Y, &data).unadjusted(),
            EPSILON
        );
        assert_float_relative_eq!(
            model.r_squared.adjusted(),
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
