use std::env;
use std::error::Error;
use std::path::PathBuf;
use anyhow::anyhow;
use clap::Parser;
use racing_scraper::models::EventType;
use tracing::{debug, info};
use bentobox::{data, fit};
use bentobox::data::{CsvFile, EventDetailExt, PredicateClosures};
use bentobox::fit::FitOptions;
use bentobox::market::{Market, OverroundMethod};
use bentobox::probs::SliceExt;

const MC_ITERATIONS_TRAIN: u64 = 100_000;
const TARGET_MSRE: f64 = 1e-6;
const OVERROUND_METHOD: OverroundMethod = OverroundMethod::Multiplicative;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// directory to source the race data from
    dir: Option<PathBuf>,

    // /// race type to analyse
    // #[clap(short = 'f', long)]
    // race_type: Option<EventType>,

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

    let mut csv = CsvFile::create(args.out.unwrap())?;
    csv.append(vec!["race_id", "runner_index", "num_runners", "places_paying", "stdev", "weight_0", "weight_1", "weight_2", "weight_3"])?;

    let mut predicates = vec![];
    predicates.push(data::Predicate::Type { race_type: EventType::Thoroughbred });
    let races = data::read_from_dir(args.dir.unwrap(), PredicateClosures::from(predicates))?;
    let races: Vec<_> = races.into_iter().map(EventDetailExt::summarise).collect();

    let podium_places = 4;
    let dilatives = vec![0.0; podium_places];
    for (index, race) in races.iter().enumerate() {
        debug!("fitting race: {race:?} ({} of {})", index + 1, races.len());
        let markets: Vec<_> = (0..race.prices.rows()).map(|rank| {
            let prices = race.prices.row_slice(rank).to_vec();
            Market::fit(&OVERROUND_METHOD, prices, rank as f64 + 1.0)
        }).collect();
        let fit_outcome = fit::fit_all(FitOptions {
            mc_iterations: MC_ITERATIONS_TRAIN,
            individual_target_msre: TARGET_MSRE,
        }, &markets, &dilatives);
        debug!("individual fitting complete: stats: {:?}, probs: \n{}", fit_outcome.stats, fit_outcome.fitted_probs.verbose());

        let num_runners = markets[0].probs.len();
        let stdev = markets[0].probs.stdev();
        for runner in 0..num_runners {
            if markets[0].probs[runner] != 0.0 {
                let mut record: Vec<String> = vec![];
                record.push(race.id.to_string());
                record.push(runner.to_string());
                record.push(num_runners.to_string());
                record.push(race.places_paying.to_string());
                record.push(stdev.to_string());
                for rank in 0..podium_places {
                    record.push(fit_outcome.fitted_probs[(rank, runner)].to_string());
                }
                debug!("{record:?}");
                csv.append(record)?;
                csv.flush()?;
            }
        }
    }
    info!("fitted {} races", races.len());

    Ok(())
}