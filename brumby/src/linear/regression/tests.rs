use assert_float_eq::*;
use ordinalizer::Ordinal;

use Regressor::{Exp, Variable, Product};

use crate::linear::regression::Regressor::{Intercept, Origin};
use crate::testing::assert_slice_f64_relative;

use super::*;

#[derive(Debug, PartialEq, ordinalizer::Ordinal, Display, Serialize, Deserialize)]
enum TestFactor {
    A,
    B,
}

impl AsIndex for TestFactor {
    fn as_index(&self) -> usize {
        self.ordinal()
    }
}

#[test]
fn serde_json() {
    fn to_json(r: &Regressor<TestFactor>) -> String {
        serde_json::to_string(&r).unwrap()
    }

    fn from_json(json: &str) -> Regressor<TestFactor> {
        serde_json::from_str(&json).unwrap()
    }

    {
        let r = Variable(TestFactor::A);
        let json = to_json(&r);
        assert_eq!(r#"{"Variable":"A"}"#, json);
        let rr = from_json(&json);
        assert_eq!(r, rr);
    }
    {
        let r = Exp(Variable(TestFactor::A).into(), 5);
        let json = to_json(&r);
        assert_eq!(r#"{"Exp":[{"Variable":"A"},5]}"#, json);
        let rr = from_json(&json);
        assert_eq!(r, rr);
    }
    {
        let r = Product(vec![Variable(TestFactor::A), Variable(TestFactor::B)]);
        let json = to_json(&r);
        assert_eq!(r#"{"Product":[{"Variable":"A"},{"Variable":"B"}]}"#, json);
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
        let r = Origin;
        let json = to_json(&r);
        assert_eq!(r#""Origin""#, json);
        let rr = from_json(&json);
        assert_eq!(r, rr);
    }
}

#[test]
fn regression_data_1() {
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
            RegressionModel::fit(Factor::Y, vec![Intercept, Variable(Factor::X)], &data).unwrap();
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
        let model = RegressionModel::fit(Factor::Y, vec![Origin, Variable(Factor::X)], &data)
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
                Variable(Factor::X),
                Exp(Variable(Factor::X).into(), 2),
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
            vec![Intercept, Variable(Factor::X), Variable(Factor::W)],
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
            vec![Origin, Variable(Factor::X), Variable(Factor::W)],
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
                Origin,
                Product(vec![Variable(Factor::X), Variable(Factor::W)]),
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
fn regression_data_2() {
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
        let mut data = Matrix::allocate(7, 3);
        data.flatten_mut()
            .clone_from_slice(&[
                1.0, 1.0, 6.0,
                3.0, 2.0, 7.0,
                4.0, 3.0, 5.0,
                5.0, 4.0, 4.0,
                2.0, 5.0, 3.0,
                3.0, 6.0, 2.0,
                4.0, 7.0, 1.0,
            ]);
        data
    }
    let data = sample_data();
    const EPSILON: f64 = 1e-13;
    {
        // with intercept
        let model =
            RegressionModel::fit(Factor::Y, vec![Intercept, Variable(Factor::X)], &data).unwrap();
        assert_slice_f64_relative(
            &model.predictor.coefficients,
            &[2.1428571428571446, 0.2499999999999994],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.std_errors,
            &[1.1406228159050942, 0.25505101530510177],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.p_values,
            &[0.11907947556121071, 0.37200486130017885],
            EPSILON,
        );
        assert_float_relative_eq!(0.16118421052631582, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(-0.006578947368421018, model.r_squared.adjusted(), EPSILON);
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
        let model = RegressionModel::fit(Factor::Y, vec![Origin, Variable(Factor::X)], &data)
            .unwrap();
        assert_slice_f64_relative(
            &model.predictor.coefficients,
            &[0.0, 0.6785714285714285],
            EPSILON,
        );
        assert_slice_f64_relative(&model.std_errors, &[0.0, 0.13599594831899833], EPSILON);
        assert_slice_f64_relative(&model.p_values, &[1.0, 0.002477775177262768], EPSILON);
        assert_float_relative_eq!(0.80580357142857142, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.7734375, model.r_squared.adjusted(), EPSILON);
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
            vec![Intercept, Variable(Factor::X), Variable(Factor::W)],
            &data,
        )
        .unwrap();
        assert_slice_f64_relative(
            &model.predictor.coefficients,
            &[-4.857142857143123, 1.1090909090909424, 0.8909090909091238],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.std_errors,
            &[7.788067081649638, 0.9801332343231696, 0.9801332343231691],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.p_values,
            &[0.5666496891788132, 0.32105658278672666, 0.4147818748530737],
            EPSILON,
        );
        assert_float_relative_eq!(0.3047846889952154, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(-0.042822966507176874, model.r_squared.adjusted(), EPSILON);
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
            vec![Origin, Variable(Factor::X), Variable(Factor::W)],
            &data,
        )
        .unwrap();
        assert_slice_f64_relative(
            &model.predictor.coefficients,
            &[0.0, 0.5046464646464648, 0.28646464646464653],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.std_errors,
            &[0.0, 0.1368908923803632, 0.1368908923803632],
            EPSILON,
        );
        assert_slice_f64_relative(
            &model.p_values,
            &[1.0, 0.014197421471356244, 0.09059457816941603],
            EPSILON,
        );
        assert_float_relative_eq!(0.8964747474747475, model.r_squared.unadjusted(), EPSILON);
        assert_float_relative_eq!(0.8550646464646465, model.r_squared.adjusted(), EPSILON);
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
