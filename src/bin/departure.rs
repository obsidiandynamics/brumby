use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::anyhow;
use clap::Parser;
use racing_scraper::models::{EventDetail, EventType};
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Cell, Col, Row, Table};
use tracing::{debug, info};

use brumby::racing_data;
use brumby::racing_data::{EventDetailExt, PlacePriceDeparture, PredicateClosures};

const TOP_SUBSET: usize = 25;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// directory to source the race data from
    dir: Option<PathBuf>,

    /// race type
    #[clap(short = 'r', long, value_parser = parse_race_type)]
    race_type: Option<EventType>,
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
        predicates.push(racing_data::Predicate::Type { race_type });
    }
    let races = racing_data::read_from_dir(args.dir.unwrap(), PredicateClosures::from(predicates))?;

    let mut assessments = Vec::with_capacity(races.len());
    let num_races = races.len();
    for (index, race_file) in races.into_iter().enumerate() {
        info!(
            "assessing race: {} ({}) ({} of {num_races})",
            race_file.race.race_name,
            race_file.file.to_str().unwrap(),
            index + 1
        );
        let departure = race_file.race.place_price_departure();
        assessments.push(Assessment {
            file: race_file.file,
            race: race_file.race,
            departure
        });
    }
    let mean_worst_departure = {
        let sum_worst_departure: f64 = assessments
            .iter()
            .map(|assessment| assessment.departure.worst)
            .sum();
        sum_worst_departure / num_races as f64
    };
    let mean_rms_departure = {
        let sum_rms_departure: f64 = assessments
            .iter()
            .map(|assessment| assessment.departure.root_mean_sq)
            .sum();
        sum_rms_departure / num_races as f64
    };
    let elapsed = start_time.elapsed();
    info!(
        "fitted {num_races} races in {}s; mean worst departure: {mean_worst_departure:.6}, mean RMS departure: {mean_rms_departure:.6}",
        elapsed.as_millis() as f64 / 1_000.
    );

    assessments.sort_by(|a, b| a.departure.worst.total_cmp(&b.departure.worst));
    let quantiles = find_quantiles(
        &assessments,
        &[0.0, 0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0],
    );
    let quantiles_table = tabulate_quantiles(&quantiles);
    info!(
        "quantiles:\n{}",
        Console::default().render(&quantiles_table)
    );

    let best_subset = &assessments[..usize::min(TOP_SUBSET, assessments.len())];
    let best_table = tabulate_subset(best_subset, 0);
    info!("best races:\n{}", Console::default().render(&best_table));

    let start_index = assessments.len().saturating_sub(TOP_SUBSET);
    let worst_subset = &assessments[start_index..];
    let worst_table = tabulate_subset(worst_subset, start_index);
    info!("worst races:\n{}", Console::default().render(&worst_table));

    Ok(())
}

fn find_quantiles(assessments: &[Assessment], quantiles: &[f64]) -> Vec<(f64, f64)> {
    let mut quantile_values = Vec::with_capacity(quantiles.len());
    for quantile in quantiles {
        let index = f64::ceil(quantile * assessments.len() as f64 - 1.) as usize;
        quantile_values.push((*quantile, assessments[index].departure.worst));
    }
    quantile_values
}

fn tabulate_subset(assessments: &[Assessment], start_index: usize) -> Table {
    let mut table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(5))),
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
                "Worst\ndeparture".into(),
                "RMS\ndeparture".into(),
                "File".into(),
                "Race\ntype".into(),
                "Places\npaying".into(),
            ],
        ));
    table.push_rows(assessments.iter().enumerate().map(|(index, assessment)| {
        Row::new(
            Styles::default(),
            vec![
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{}", index + start_index + 1).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.6}", assessment.departure.worst).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.6}", assessment.departure.root_mean_sq).into(),
                ),
                Cell::new(
                    Styles::default(),
                    assessment.file.to_str().unwrap().to_string().into(),
                ),
                Cell::new(
                    Styles::default(),
                    format!("{}", assessment.race.race_type).into(),
                ),
                Cell::new(
                    Styles::default().with(HAlign::Right),
                    format!("{:.6}", assessment.race.places_paying).into(),
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
            Col::new(Styles::default().with(MinWidth(14))),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Quantile".into(), "Worst departure".into()],
        ));
    table.push_rows(quantiles.iter().map(|(quantile, worst_departure)| {
        Row::new(
            Styles::default().with(HAlign::Right),
            vec![
                format!("{quantile:.2}").into(),
                format!("{worst_departure:.6}").into(),
            ],
        )
    }));
    table
}

#[derive(Debug)]
struct Assessment {
    file: PathBuf,
    race: EventDetail,
    departure: PlacePriceDeparture,
}
