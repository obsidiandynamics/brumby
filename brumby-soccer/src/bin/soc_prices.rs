use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use rustc_hash::FxHashMap;
use tracing::{debug, info};

use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, OverroundMethod, PriceBounds};
use brumby::probs::SliceExt;
use brumby_soccer::data::{download_by_id, ContestSummary, SoccerFeedId};
use brumby_soccer::domain::{Offer, OfferType, OutcomeType, Over, Period};
use brumby_soccer::model::{Model, score_fitter};
use brumby_soccer::model::score_fitter::ScoreFitter;

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.01..=301.0;
const FIRST_GOALSCORER_BOOKSUM: f64 = 1.5;
const INTERVALS: usize = 18;
// const MAX_TOTAL_GOALS_HALF: u16 = 4;
const MAX_TOTAL_GOALS_FULL: u16 = 8;
const GOALSCORER_MIN_PROB: f64 = 0.0;
// const ERROR_TYPE: ErrorType = ErrorType::SquaredRelative;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// file to source the contest data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download contest data by ID
    #[clap(short = 'd', long)]
    // download: Option<String>,
    download: Option<SoccerFeedId>,

    /// print player goal markets
    #[clap(long = "player-goals")]
    player_goals: bool,

    /// print player assists markets
    #[clap(long = "player-assists")]
    player_assists: bool,
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

const INCREMENTAL_OVERROUND: f64 = 0.01;

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
    let contest = read_contest_data(&args).await?;
    info!("contest.name: {}", contest.name);

    let offers = contest
        .offerings
        .iter()
        .map(|(offer_type, prices)| {
            debug!("sourced {offer_type:?} with {} outcomes, σ={:.3}", prices.len(), implied_booksum(prices.values()));
            let normal = match &offer_type {
                OfferType::HeadToHead(_)
                | OfferType::TotalGoals(_, _)
                | OfferType::CorrectScore(_)
                | OfferType::DrawNoBet => 1.0,
                OfferType::AnytimeGoalscorer
                | OfferType::FirstGoalscorer
                | OfferType::PlayerShotsOnTarget(_)
                | OfferType::AnytimeAssist => {
                    let implied_booksum = implied_booksum(prices.values());
                    let expected_overround = prices.len() as f64 * INCREMENTAL_OVERROUND;
                    implied_booksum / expected_overround
                },
            };
            let offer = fit_offer(offer_type.clone(), prices, normal);
            (offer_type.clone(), offer)
        })
        .collect::<FxHashMap<_, _>>();

    let mut model = Model::new();
    let score_fitter = ScoreFitter::try_from(score_fitter::Config::default())?;
    score_fitter.fit(&mut model, &offers)?;

    Ok(())
}

fn implied_booksum<'a>(prices: impl Iterator<Item = &'a f64>) -> f64 {
    prices.map(|&price| 1.0 / price).sum()
}

fn fit_offer(offer_type: OfferType, map: &HashMap<OutcomeType, f64>, normal: f64) -> Offer {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let outcomes = entries
        .iter()
        .map(|(outcome, _)| (*outcome).clone())
        .collect::<Vec<_>>();
    let prices = entries.iter().map(|(_, &price)| price).collect();
    let market = Market::fit(&OVERROUND_METHOD, prices, normal);
    Offer {
        offer_type,
        outcomes: HashLookup::from(outcomes),
        market,
    }
}

async fn read_contest_data(args: &Args) -> anyhow::Result<ContestSummary> {
    let contest = {
        if let Some(_) = args.file.as_ref() {
            //ContestModel::read_json_file(path)?
            unimplemented!()
        } else if let Some(id) = args.download.as_ref() {
            download_by_id(id.clone()).await?
        } else {
            unreachable!()
        }
    };
    Ok(contest.into())
}
