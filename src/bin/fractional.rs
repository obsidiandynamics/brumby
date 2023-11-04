use std::env;
use std::error::Error;
use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use racing_scraper::models::EventDetail;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};
use tracing::{debug, info};

use brumby::data::{download_by_id, EventDetailExt, RaceSummary};
use brumby::file::ReadJsonFile;
use brumby::market::{Market, Overround, OverroundMethod};
use brumby::model::{fit, PODIUM, TopN};
use brumby::model::fit::compute_msre;
use brumby::print::{tabulate_derived_prices, tabulate_prices, tabulate_values};

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// file to source the race data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download race data by ID
    #[clap(short = 'd', long)]
    download: Option<u64>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        if self.file.is_none() && self.download.is_none()
            || self.file.is_some() && self.download.is_some()
        {
            bail!("either the -f or the -d flag must be specified");
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    let race = read_race_data(&args).await?;
    debug!(
        "meeting: {}, race: {}, places_paying: {}, prices: {:?}",
        race.meeting_name, race.race_number, race.places_paying, race.prices,
    );

    let sample_top_n = TopN {
        markets: (0..race.prices.rows())
            .map(|rank| {
                let prices = race.prices.row_slice(rank).to_vec();
                Market::fit(&OverroundMethod::Fractional, prices, rank as f64 + 1.)
            })
            .collect(),
    };
    let sample_overrounds = sample_top_n.overrounds()?;

    let implied_probs: Vec<_> = race.prices[0]
        .iter()
        .map(|price| 1. / price)
        .collect();
    let fitted_top_n = TopN {
        markets: (0..PODIUM)
            .into_iter()
            .map(|rank| {
                Market::frame(
                    &Overround {
                        method: OverroundMethod::Fractional,
                        value: (rank + 1) as f64 * sample_overrounds[rank].value
                            / sample_overrounds[0].value,
                    },
                    implied_probs.clone(),
                )
            })
            .collect()
    };

    // for (rank, fractional_market) in fitted_top_n.markets.iter().enumerate() {
    //     let booksum: f64 = fractional_market.prices.iter().map(|price| 1. / price).sum();
    //     let rmsre = compute_msre(
    //         &sample_top_n.markets[rank].prices,
    //         &fractional_market.prices,
    //         &fit::FITTED_PRICE_RANGES[rank],
    //     )
    //     .sqrt();
    //     debug!(
    //         "fractional: {}, booksum: {:.6}, rmsre: {:.6}",
    //         DisplaySlice::from(&*fractional_market.prices),
    //         booksum,
    //         rmsre,
    //     );
    // }

    let derived_prices = fitted_top_n.as_price_matrix();
    let table = tabulate_derived_prices(&derived_prices);
    info!("\n{}", Console::default().render(&table));

    let errors: Vec<_> = (0..derived_prices.rows())
        .map(|rank| {
            compute_msre(
                &race.prices[rank],
                &derived_prices[rank],
                &fit::FITTED_PRICE_RANGES[rank],
            )
                .sqrt()
        })
        .collect();

    let errors_table = tabulate_values(&errors, "RMSRE");
    let sample_overrounds: Vec<_> = sample_overrounds
        .iter()
        .map(|overround| overround.value)
        .collect();

    let overrounds_table = tabulate_values(&sample_overrounds, "Overround");
    let sample_prices_table = tabulate_prices(&race.prices);
    let summary_table = Table::with_styles(Styles::default().with(HAlign::Centred))
        .with_cols(vec![
            Col::default(),
            Col::new(Styles::default().with(Separator(true)).with(MinWidth(9))),
            Col::default(),
            Col::new(Styles::default().with(Separator(true)).with(MinWidth(10))),
            Col::default(),
        ])
        .with_row(Row::from([
            "Fitting errors",
            "",
            "Fitted overrounds",
            "",
            "Sample prices",
        ]))
        .with_row(Row::new(
            Styles::default(),
            vec![
                errors_table.into(),
                "".into(),
                overrounds_table.into(),
                "".into(),
                sample_prices_table.into(),
            ],
        ));
    info!("\n{}", Console::default().render(&summary_table));

    // if let Some(selections) = args.selections {
    //     let price = model.derive_multi(&selections)?;
    //     info!(
    //         "probability of {}: {}, fair price: {:.3}, overround: {:.3}, market odds: {:.3}",
    //         DisplaySlice::from(&*selections),
    //         price.value.probability,
    //         price.value.fair_price(),
    //         price.value.fair_price() / price.value.price,
    //         price.value.price
    //     );
    //     debug!(
    //         "price generation took {:.3}s",
    //         price.elapsed.as_millis() as f64 / 1_000.
    //     );
    // }
    Ok(())
}

async fn read_race_data(args: &Args) -> anyhow::Result<RaceSummary> {
    if let Some(path) = args.file.as_ref() {
        let event_detail = EventDetail::read_json_file(path)?;
        return Ok(event_detail.summarise());
    }
    if let Some(&id) = args.download.as_ref() {
        let event_detail = download_by_id(id).await?;
        return Ok(event_detail.summarise());
    }
    unreachable!()
}
