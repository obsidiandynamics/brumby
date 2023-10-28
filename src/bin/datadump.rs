use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::anyhow;
use clap::Parser;
use racing_scraper::models::EventType;
use strum::{EnumCount, IntoEnumIterator};
use tracing::{debug, info};
use brumby::{data};
use brumby::csv::{CsvWriter, Record};
use brumby::data::{EventDetailExt, Factor, PredicateClosures};
use brumby::market::{Market, OverroundMethod};
use brumby::model::fit;
use brumby::model::fit::FitOptions;
use brumby::probs::SliceExt;

const MC_ITERATIONS_TRAIN: u64 = 100_000;
const TARGET_MSRE: f64 = 1e-6;
const OVERROUND_METHOD: OverroundMethod = OverroundMethod::Multiplicative;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// directory to source the race data from
    dir: Option<PathBuf>,

    /// race type
    #[clap(short = 'r', long, value_parser = parse_race_type)]
    race_type: Option<EventType>,

    /// where to write the CSV to
    out: Option<PathBuf>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        self.dir.as_ref().ok_or(anyhow!("data directory must be specified"))?;
        self.out.as_ref().ok_or(anyhow!("output file must be specified"))?;
        Ok(())
    }
}
fn parse_race_type(s: &str) -> anyhow::Result<EventType> {
    match s.to_lowercase().as_str() {
        "t" | "thoroughbred" => Ok(EventType::Thoroughbred),
        "g" | "greyhound" => Ok(EventType::Thoroughbred),
        _ => Err(anyhow!("unsupported race type {s}")),
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

    let start_time = Instant::now();
    let mut csv = CsvWriter::create(args.out.unwrap())?;
    csv.append(Record::with_values(Factor::iter()))?;

    let mut predicates = vec![];
    if let Some(race_type) = args.race_type {
        predicates.push(data::Predicate::Type { race_type });
    }
    let races = data::read_from_dir(args.dir.unwrap(), PredicateClosures::from(predicates))?;
    let races: Vec<_> = races.into_iter().map(EventDetailExt::summarise).collect();

    for (index, race) in races.iter().enumerate() {
        debug!("fitting race: {race:?} ({} of {})", index + 1, races.len());
        let markets: Vec<_> = (0..race.prices.rows()).map(|rank| {
            let prices = race.prices.row_slice(rank).to_vec();
            Market::fit(&OVERROUND_METHOD, prices, rank as f64 + 1.0)
        }).collect();
        let fit_outcome = fit::fit_all(FitOptions {
            mc_iterations: MC_ITERATIONS_TRAIN,
            individual_target_msre: TARGET_MSRE,
        }, &markets);
        debug!("individual fitting complete: stats: {:?}, probs: \n{}", fit_outcome.stats, fit_outcome.fitted_probs.verbose());

        let num_runners = markets[0].probs.len();
        let active_runners = markets[0].probs.iter().filter(|&&prob| prob != 0.).count();
        let stdev = markets[0].probs.stdev();
        for runner in 0..num_runners {
            if markets[0].probs[runner] != 0.0 {
                let mut record = Record::with_capacity(Factor::COUNT);
                record.set(Factor::RaceId, race.id);
                record.set(Factor::RunnerIndex, runner);
                record.set(Factor::ActiveRunners, active_runners);
                record.set(Factor::PlacesPaying, race.places_paying);
                record.set(Factor::Stdev, stdev);
                record.set(Factor::Weight0, fit_outcome.fitted_probs[(0, runner)]);
                record.set(Factor::Weight1, fit_outcome.fitted_probs[(1, runner)]);
                record.set(Factor::Weight2, fit_outcome.fitted_probs[(2, runner)]);
                record.set(Factor::Weight3, fit_outcome.fitted_probs[(3, runner)]);
                debug!("{record:?}");
                csv.append(record)?;
                csv.flush()?;
            }
        }
    }
    let elapsed_time = start_time.elapsed();
    info!("fitted {} races in {}s", races.len(), elapsed_time.as_millis() as f64 / 1_000.);

    Ok(())
}