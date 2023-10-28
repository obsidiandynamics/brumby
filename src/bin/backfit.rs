use std::env;
use std::error::Error;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
use linregress::fit_low_level_regression_model;
use strum::{EnumCount, IntoEnumIterator};
use tracing::{debug, info};

use brumby::csv::CsvReader;
use brumby::data::Factor;
use brumby::data::Factor::{ActiveRunners, Weight0};
use brumby::linear::matrix::Matrix;
use brumby::linear::regression;
use brumby::linear::regression::Regressor::{Exponent, NilIntercept, Ordinal};

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// dataset to analyse
    file: Option<PathBuf>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        self.file
            .as_ref()
            .ok_or(anyhow!("input file must be specified"))?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if env::var("RUST_BACKTRACE").is_err() {
        env::set_var("RUST_BACKTRACE", "full")
    }
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    args.validate()?;
    debug!("args: {args:?}");

    // let data_row_major: Vec<f64> = vec![
    //     1., 0.0, 1., 7.,
    //     3., 0.0, 2., 6.,
    //     4., 0.0, 3., 5.,
    //     5., 0.0, 4., 4.,
    //     2., 0.0, 5., 3.,
    //     3., 0.0, 6., 2.,
    //     4., 0.0, 7., 1.,
    // ];
    //
    // let model = fit_low_level_regression_model(&data_row_major, 7, 4)?;
    // println!("params: {:?}", model.parameters());
    // println!("std_errors: {:?}", model.se());
    // println!("p_values: {:?}", model.p_values());
    // println!("r_squared: {}", model.rsquared());
    // println!("r_squared_adj: {}", model.rsquared_adj());
    // let params = [
    //     0.09523809523809518f64,
    //     0.5059523809523807,
    //     0.2559523809523811,
    // ];

    // #[derive(Debug, Default)]
    // struct Data {
    //     runners: Vec<f64>,
    //     stdev: Vec<f64>,
    //     weight_0: Vec<f64>,
    //     weight_1: Vec<f64>,
    //     weight_2: Vec<f64>,
    //     weight_3: Vec<f64>,
    // }

    let mut csv = CsvReader::open(args.file.unwrap())?;
    // let mut data = Vec::with_capacity(Factor::COUNT);
    // data.resize_with(data.capacity(), || vec![]);
    // let mut data = Data::default();

    let _header = csv.next().unwrap();
    let records: Vec<_> = csv.collect();
    let mut data = Matrix::allocate(records.len(), Factor::COUNT);
    for (record_index, record) in records.into_iter().enumerate() {
        let record = record?;
        for factor in Factor::iter() {
            let value = record[factor.ordinal()].parse::<f64>()?;
            data[(record_index, factor.ordinal())] = value;
        }
    }

    let regressors = vec![
        Ordinal(Weight0),
        Exponent(Box::new(Ordinal(Weight0)), 2),
        Exponent(Box::new(Ordinal(Weight0)), 3),
        Ordinal(ActiveRunners),
        Exponent(Box::new(Ordinal(ActiveRunners)), 2),
        Exponent(Box::new(Ordinal(ActiveRunners)), 3),
        NilIntercept
    ];
    let model = regression::fit(Factor::Weight1, regressors, &data)?;
    info!("model:\n{:#?}", model);

    // let records: Vec<_> = csv.collect();
    // let rows = records.len();
    // let cols = 4; // response + regressors (incl. intercept)
    // let mut data = Matrix::allocate(rows, cols);
    // for (row, record) in records.into_iter().enumerate() {
    //     let record = record?;
    //     let mut cols = (0..cols).into_iter();
    //     data[(row, cols.next().unwrap())] = record[Factor::Weight1].parse::<f64>()?;
    //     data[(row, cols.next().unwrap())] = 0.;
    //     data[(row, cols.next().unwrap())] = record[Factor::Weight0].parse::<f64>()?;
    //     data[(row, cols.next().unwrap())] = record[Factor::Weight2].parse::<f64>()?;
    // }
    //
    // let model = fit_low_level_regression_model(data.flatten(), rows, cols)?;
    // println!("params: {:?}", model.parameters());
    // println!("std_errors: {:?}", model.se());
    // println!("p_values: {:?}", model.p_values());
    // println!("r_squared: {}", model.rsquared());
    // println!("r_squared_adj: {}", model.rsquared_adj());
    //
    // let mut ssr = 0.;
    // for row in 0..rows {
    //     let weight_1 = data[(row, 0)];
    //     let weight_0 = data[(row, 2)];
    //     let weight_2 = data[(row, 3)];
    //     let predicted = weight_0 * 0.3716956118305484 + weight_2 * 0.652003535928119;
    //     let residual = predicted - weight_1;
    //     ssr += residual.powi(2);
    // }
    // let mut sum = 0.;
    // for row in 0..rows {
    //     let weight_1 = data[(row, 0)];
    //     sum += weight_1;
    // }
    // let mean = sum / rows as f64;
    // let mut sst = 0.;
    // for row in 0..rows {
    //     let weight_1 = data[(row, 0)];
    //     let residual = weight_1 - mean;
    //     sst += residual.powi(2);
    // }
    // let r_squared = 1. - ssr / sst;
    // println!("r_squared (alternative): {r_squared}");

    // for record in csv {
    //     let record = record?;
    //     for factor in Factor::iter() {
    //         let value = record[factor.ordinal()].parse::<f64>()?;
    //         data[factor.ordinal()].push(value);
    //     }
    // }

    // let factors: Vec<_> = Factor::iter().collect();
    // let data: Vec<_> = data.into_iter().enumerate().map(|(ordinal, values)| {
    //     (factors[ordinal].to_string(), values)
    // }).collect();
    //
    // let mut sum_resid_sq = 0.;
    // let count = data[0].1.len();
    // for i in 0..count {
    //     let weight_0 = data[Factor::Weight0.ordinal()].1[i];
    //     let weight_1 = data[Factor::Weight1.ordinal()].1[i];
    //     let predicted = weight_0 * 0.751864069553756 + 0.025308764524450442;
    //     let residual = predicted - weight_1;
    //     sum_resid_sq += residual.powi(2);
    // }
    // println!("sum_resid_sq: {sum_resid_sq}");
    // let mean = data[Factor::Weight1.ordinal()].1.mean();
    // let mut sum_mean_sq = 0.;
    // for i in 0..count {
    //     let mean_diff = data[Factor::Weight1.ordinal()].1[i] - mean;
    //     sum_mean_sq += mean_diff.powi(2);
    // }
    // println!("sum_mean_sq {sum_mean_sq}");
    // println!("mean: {mean}, rsquared: {}", 1. - sum_resid_sq / sum_mean_sq);
    //
    // // println!("data={:?}", data[Factor::Weight0.ordinal()]);
    // let data = RegressionDataBuilder::new().build_from(data)?;
    // let formula = "Weight1 ~ Weight0";
    // let model = FormulaRegressionBuilder::new()
    //     .data(&data)
    //     // .formula(formula)
    //     .data_columns(Cow::Borrowed("Weight1"),vec![Cow::Borrowed("Weight0")])
    //     .fit()?;
    //
    // let parameters: Vec<_> = model.iter_parameter_pairs().collect();
    // let p_values: Vec<_> = model.iter_p_value_pairs().collect();
    // let standard_errors: Vec<_> = model.iter_se_pairs().collect();
    // println!("intercept: {}", model.parameters()[0]);
    // println!("parameters: {parameters:?}");
    // println!("p_values: {p_values:?}");
    // println!("standard_errors: {standard_errors:?}");
    // println!("r_sq: {}", model.rsquared());

    // println!("residuals: {:?}", model.residuals());

    // let data: Vec<_> = Factor::iter().map(|factor| {
    //     (factor.to_string(), data[factor.ordinal()].clone())
    // }).collect();

    // let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3), ("B", b)];

    // let y = vec![1., 2. ,3. , 4., 5.];
    // let x1 = vec![5., 4., 3., 2., 1.];
    // let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
    // let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
    // let b = vec![1., 1. ,1. , 1., 1.];
    // let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3), ("B", b)];
    // let data = RegressionDataBuilder::new().build_from(data)?;
    // let formula = "Y ~ X1 + X2 + X3 + B";
    // let model = FormulaRegressionBuilder::new()
    //     .data(&data)
    //     .formula(formula)
    //     .fit()?;
    // let parameters: Vec<_> = model.iter_parameter_pairs().collect();
    // let pvalues: Vec<_> = model.iter_p_value_pairs().collect();
    // let standard_errors: Vec<_> = model.iter_se_pairs().collect();
    // println!("parameters: {parameters:?}");
    // println!("pvalues: {pvalues:?}");
    // println!("standard_errors: {standard_errors:?}");
    // assert_eq!(
    //     parameters,
    //     vec![
    //         ("X1", -0.9999999999999745),
    //         ("X2", 1.5872719805187785e-15),
    //         ("X3", -1.4246416546459528e-15),
    //     ]
    // );
    // assert_eq!(
    //     standard_errors,
    //     vec![
    //         ("X1", 9.799066977595267e-13),
    //         ("X2", 4.443774660560714e-15),
    //         ("X3", 2.713389610740135e-15),
    //     ]
    // );
    // assert_eq!(
    //     pvalues,
    //     vec![
    //         ("X1", 6.238279788691533e-13),
    //         ("X2", 0.7815975465725482),
    //         ("X3", 0.6922074604135647),
    //     ]
    // );

    Ok(())
}
