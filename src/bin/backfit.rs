use std::env;
use std::error::Error;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use strum::{EnumCount, IntoEnumIterator};
use tracing::{debug, info};

use brumby::csv::CsvReader;
use brumby::file::{ReadJsonFile, WriteJsonFile};
use brumby::linear::matrix::Matrix;
use brumby::linear::regression::{RegressionModel, Regressor};
use brumby::model::cf::{Coefficients, Factor, Regressors};

/// Fits a linear regression model to the given dataset
#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// dataset to analyse
    input: Option<PathBuf>,

    /// path to the regressors file
    regressors: Option<PathBuf>,

    /// output file for the fitted coefficients
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        self.input
            .as_ref()
            .ok_or(anyhow!("input file must be specified"))?;
        self.input
            .as_ref()
            .ok_or(anyhow!("regressors file must be specified"))?;
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

    let regressors = Regressors::read_json_file(args.regressors.unwrap())?;
    regressors.validate()?;
    debug!("regressors:\n{regressors:#?}");

    let mut csv = CsvReader::open(args.input.unwrap())?;
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

    let w1 = fit_linear_model(Factor::Weight1, regressors.w1, &data)?;
    let w2 = fit_linear_model(Factor::Weight2, regressors.w2, &data)?;
    let w3 = fit_linear_model(Factor::Weight3, regressors.w3, &data)?;
    let coefficients = Coefficients {
        w1: w1.predictor,
        w2: w2.predictor,
        w3: w3.predictor,
    };
    debug!("fitted coefficients:\n{coefficients:#?}");

    if let Some(output) = args.output {
        coefficients.write_json_file(output)?;
    }

    Ok(())
}

fn fit_linear_model(
    response: Factor,
    regressors: Vec<Regressor<Factor>>,
    data: &Matrix<f64>,
) -> Result<RegressionModel<Factor>, anyhow::Error> {
    info!("fitting linear model for {response:?}...");
    let model = RegressionModel::fit(response, regressors, data)?;
    let table = model.tabulate();
    info!(
        "fitted model:\n{}\nr_squared:     {:.6}\nr_squared_adj: {:.6}",
        Console::default().render(&table),
        model.r_squared,
        model.r_squared_adj
    );
    Ok(model)
}
