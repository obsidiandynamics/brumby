use anyhow::anyhow;
use brumby::csv::{CsvWriter, Record};
use brumby::{data, model};
use brumby::data::{EventDetailExt, PredicateClosures};
use brumby::market::{Market, OverroundMethod};
use brumby::model::cf::Factor;
use brumby::model::fit;
use brumby::model::fit::FitOptions;
use brumby::probs::SliceExt;
use clap::Parser;
use racing_scraper::models::EventType;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use strum::{EnumCount, IntoEnumIterator};
use tracing::{debug, info};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::Multiplicative;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// directory to source the race data from
    dir: Option<PathBuf>,

    /// race type
    #[clap(short = 'r', long, value_parser = parse_race_type)]
    race_type: Option<EventType>,

    /// cutoff place price departure
    #[clap(short = 'd', long)]
    departure: Option<f64>,

    /// where to write the CSV to
    out: Option<PathBuf>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        self.dir
            .as_ref()
            .ok_or(anyhow!("data directory must be specified"))?;
        self.out
            .as_ref()
            .ok_or(anyhow!("output file must be specified"))?;
        Ok(())
    }
}
fn parse_race_type(s: &str) -> anyhow::Result<EventType> {
    match s.to_lowercase().as_str() {
        "t" | "thoroughbred" => Ok(EventType::Thoroughbred),
        "g" | "greyhound" => Ok(EventType::Greyhound),
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
    if let Some(cutoff_worst) = args.departure {
        predicates.push(data::Predicate::Departure { cutoff_worst })
    }
    let race_files = data::read_from_dir(args.dir.unwrap(), PredicateClosures::from(predicates))?;
    let total_num_races = race_files.len();
    let mut unique_races = HashSet::new();
    let mut duplicate_races = 0;
    for (index, race_file) in race_files.into_iter().enumerate() {
        if !unique_races.insert(race_file.race.id) {
            info!("skipping duplicate race {}", race_file.race.id);
            duplicate_races += 1;
            continue;
        }
        info!(
            "fitting race: {} ({}) ({} of {total_num_races})",
            race_file.race.race_name,
            race_file.file.to_str().unwrap(),
            index + 1,
        );
        let race = race_file.race.summarise();
        let markets: Vec<_> = (0..race.prices.rows())
            .map(|rank| {
                let prices = race.prices.row_slice(rank).to_vec();
                Market::fit(&OVERROUND_METHOD, prices, rank as f64 + 1.0)
            })
            .collect();
        let fit_outcome = fit::fit_all(&FitOptions::default(), &markets, &model::SINGLE_PRICE_BOUNDS)?;
        debug!(
            "individual fitting complete: stats: {:?}, probs: \n{}",
            fit_outcome.stats,
            fit_outcome.fitted_probs.verbose()
        );

        let runners = markets[0].probs.len();
        let active_runners = markets[0].probs.iter().filter(|&&prob| prob != 0.).count();
        let stdev = markets[0].probs.stdev();
        for runner in 0..runners {
            if markets[0].probs[runner] != 0.0 {
                let mut record = Record::with_capacity(Factor::COUNT);
                record.set(Factor::RaceId, &race.id);
                record.set(Factor::RunnerIndex, &runner);
                record.set(Factor::ActiveRunners, &active_runners);
                record.set(Factor::PlacesPaying, &race.places_paying);
                record.set(Factor::Stdev, &stdev);
                record.set(Factor::Weight0, &fit_outcome.fitted_probs[(0, runner)]);
                record.set(Factor::Weight1, &fit_outcome.fitted_probs[(1, runner)]);
                record.set(Factor::Weight2, &fit_outcome.fitted_probs[(2, runner)]);
                record.set(Factor::Weight3, &fit_outcome.fitted_probs[(3, runner)]);
                debug!("{record:?}");
                csv.append(record)?;
                csv.flush()?;
            }
        }
    }
    let elapsed = start_time.elapsed();
    info!(
        "fitted {} races in {}s; {duplicate_races} duplicates ignored",
        total_num_races - duplicate_races,
        elapsed.as_millis() as f64 / 1_000.
    );

    Ok(())
}
