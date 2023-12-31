use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::anyhow;
use clap::Parser;
use racing_scraper::racing::sports_bet::models::EventType;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Cell, Col, Row, Table};
use tracing::{debug, info};

use brumby_racing::data;
use brumby_racing::data::{EventDetailExt, PlacePriceDeparture, PredicateClosures, RaceSummary};
use brumby::file::ReadJsonFile;
use brumby::market::{Market, OverroundMethod};
use brumby_racing::model::cf::Coefficients;
use brumby_racing::model::{fit, Fitter, FitterConfig, TopN, WinPlace};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::Multiplicative;
const TOP_SUBSET: usize = 25;

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
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        self.dir
            .as_ref()
            .ok_or(anyhow!("data directory must be specified"))?;
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
    let mut predicates = vec![];
    if let Some(race_type) = args.race_type {
        predicates.push(data::Predicate::Type { race_type });
    }
    if let Some(cutoff_worst) = args.departure {
        predicates.push(data::Predicate::Departure { cutoff_worst })
    }
    let races = data::read_from_dir(args.dir.unwrap(), PredicateClosures::from(predicates))?;

    let mut configs = HashMap::new();
    for race_type in [EventType::Thoroughbred, EventType::Greyhound] {
        let filename = match race_type {
            EventType::Thoroughbred => "brumby-racing/config/thoroughbred.cf.json",
            EventType::Greyhound => "brumby-racing/config/greyhound.cf.json",
            EventType::Harness => unimplemented!(),
        };
        debug!("loading {race_type} config from {filename}");
        let config = FitterConfig {
            coefficients: Coefficients::read_json_file(filename)?,
            fit_options: Default::default(),
        };
        configs.insert(race_type, config);
    }

    let mut evaluations = Vec::with_capacity(races.len());
    let total_num_races = races.len();
    let mut unique_races = HashSet::new();
    let mut duplicate_races = 0;
    for (index, race_file) in races.into_iter().enumerate() {
        if !unique_races.insert(race_file.race.id) {
            info!("skipping duplicate race {}", race_file.race.id);
            duplicate_races += 1;
            continue;
        }
        info!(
            "fitting race: {} ({}) ({} of {total_num_races})",
            race_file.race.race_name,
            race_file.file.to_str().unwrap(),
            index + 1
        );
        let departure = race_file.race.place_price_departure();
        let race = RaceSummary::from(race_file.race);
        let calibrator = Fitter::try_from(configs[&race.race_type].clone())?;
        let sample_top_n = TopN {
            markets: (0..race.prices.rows())
                .map(|rank| {
                    let prices = race.prices.row_slice(rank).to_vec();
                    Market::fit(&OVERROUND_METHOD, prices, rank as f64 + 1.)
                })
                .collect(),
        };
        let sample_wp = WinPlace {
            win: sample_top_n.markets[0].clone(),
            place: sample_top_n.markets[race.places_paying - 1].clone(),
            places_paying: race.places_paying,
        };
        let sample_overrounds = sample_top_n.overrounds()?;
        let model = calibrator.fit(&sample_wp, &sample_overrounds)?.value;
        let derived_prices = model.top_n.as_price_matrix();
        let errors: Vec<_> = (0..derived_prices.rows())
            .map(|rank| {
                fit::compute_msre(
                    &race.prices[rank],
                    &derived_prices[rank],
                    &fit::FITTED_PRICE_RANGES[rank],
                )
                .sqrt()
            })
            .collect();
        let worst_rmsre = *errors.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        debug!("worst_rmsre: {worst_rmsre}");
        evaluations.push(Evaluation {
            file: race_file.file,
            race,
            worst_rmsre,
            departure,
        });
    }
    let mean_worst_rmsre = {
        let sum_rmsre: f64 = evaluations
            .iter()
            .map(|evaluation| evaluation.worst_rmsre)
            .sum();
        sum_rmsre / (total_num_races - duplicate_races) as f64
    };
    let elapsed = start_time.elapsed();
    info!(
        "fitted {} races in {}s; mean worst RMSRE: {mean_worst_rmsre:.6}; {duplicate_races} duplicates ignored",
        total_num_races - duplicate_races,
        elapsed.as_millis() as f64 / 1_000.
    );

    evaluations.sort_by(|a, b| a.worst_rmsre.total_cmp(&b.worst_rmsre));
    let quantiles = find_quantiles(
        &evaluations,
        &[0.0, 0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0],
    );
    let quantiles_table = tabulate_quantiles(&quantiles);
    info!(
        "quantiles:\n{}",
        Console::default().render(&quantiles_table)
    );

    let best_subset = &evaluations[..usize::min(TOP_SUBSET, evaluations.len())];
    let best_table = tabulate_subset(best_subset, 0);
    info!("best races:\n{}", Console::default().render(&best_table));

    let start_index = evaluations.len().saturating_sub(TOP_SUBSET);
    let worst_subset = &evaluations[start_index..];
    let worst_table = tabulate_subset(worst_subset, start_index);
    info!("worst races:\n{}", Console::default().render(&worst_table));

    Ok(())
}

fn find_quantiles(evaluations: &[Evaluation], quantiles: &[f64]) -> Vec<(f64, f64)> {
    let mut quantile_values = Vec::with_capacity(quantiles.len());
    for quantile in quantiles {
        let index = f64::ceil(quantile * evaluations.len() as f64 - 1.) as usize;
        quantile_values.push((*quantile, evaluations[index].worst_rmsre));
    }
    quantile_values
}

fn tabulate_subset(evaluations: &[Evaluation], start_index: usize) -> Table {
    let mut table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(5))),
            Col::new(Styles::default()),
            Col::new(Styles::default()),
            Col::new(Styles::default()),
            Col::new(Styles::default().with(MinWidth(60))),
            Col::new(Styles::default()),
            Col::new(Styles::default()),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec![
                "Rank".into(),
                "Worst\nRMSRE".into(),
                "Worst\ndeparture".into(),
                "RMS\ndeparture".into(),
                "File".into(),
                "Race\ntype".into(),
                "Places\npaying".into(),
            ],
        ));
    table.push_rows(evaluations.iter().enumerate().map(|(index, evaluation)| {
        Row::new(
            Styles::default(),
            vec![
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{}", index + start_index + 1).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.6}", evaluation.worst_rmsre).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.3}", evaluation.departure.worst).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.3}", evaluation.departure.root_mean_sq).into(),
                ),
                Cell::new(
                    Styles::default(),
                    evaluation.file.to_str().unwrap().to_string().into(),
                ),
                Cell::new(
                    Styles::default(),
                    format!("{}", evaluation.race.race_type).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.6}", evaluation.race.places_paying).into(),
                ),
            ],
        )
    }));

    table
}

fn tabulate_quantiles(quantiles: &[(f64, f64)]) -> Table {
    let mut table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10))),
            Col::new(Styles::default().with(MinWidth(12))),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Quantile".into(), "Worst RMSRE".into()],
        ));
    table.push_rows(quantiles.iter().map(|(quantile, rmsre)| {
        Row::new(
            Styles::default().with(HAlign::Right),
            vec![
                format!("{quantile:.2}").into(),
                format!("{rmsre:.6}").into(),
            ],
        )
    }));
    table
}

#[derive(Debug)]
struct Evaluation {
    file: PathBuf,
    race: RaceSummary,
    worst_rmsre: f64,
    departure: PlacePriceDeparture
}
