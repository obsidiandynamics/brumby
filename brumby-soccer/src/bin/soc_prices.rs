use std::collections::{BTreeMap, HashMap};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{bail, Context};
use brumby::derived_price::DerivedPrice;
use clap::Parser;
use regex::Regex;
use rustc_hash::FxHashMap;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::Styles;
use stanza::table::{Cell, Content, Row, Table};
use tracing::{debug, info, warn};

use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, OverroundMethod, PriceBounds};
use brumby::tables;
use brumby::timed::Timed;
use brumby_soccer::data::{download_by_id, ContestSummary, SoccerFeedId};
use brumby_soccer::domain::{Offer, OfferType, Outcome};
use brumby_soccer::fit::{ErrorType, FittingErrors};
use brumby_soccer::model::player_assist_fitter::PlayerAssistFitter;
use brumby_soccer::model::player_goal_fitter::PlayerGoalFitter;
use brumby_soccer::model::score_fitter::ScoreFitter;
use brumby_soccer::model::{score_fitter, FitError, Model, Stub};
use brumby_soccer::{fit, model, print};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.001..=301.0;
const MULTI_PRICE_BOUNDS: PriceBounds = 1.001..=1001.0;
const INTERVALS: u8 = 8;
const INCREMENTAL_OVERROUND: f64 = 0.01;
const MAX_TOTAL_GOALS: u16 = 18;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// file to source the contest data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download contest data by ID
    #[clap(short = 'd', long)]
    download: Option<SoccerFeedId>,

    /// print player goal markets
    #[clap(long = "player-goals")]
    player_goals: bool,

    /// print player assists markets
    #[clap(long = "player-assists")]
    player_assists: bool,

    /// JSON file containing the selections to price
    selections: Option<String>,
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

fn load_selections(filename: &str) -> anyhow::Result<Vec<(OfferType, Outcome)>> {
    let file = File::open(filename).context(format!("opening file '{filename}'"))?;
    let reader = BufReader::new(file);
    let mut contents = String::new();
    let comment = Regex::new(r"^.*(#|//)")?;
    for line in reader.lines() {
        let line = line?;
        if !comment.is_match(&line) {
            contents.push_str(&line);
        }
    }
    let selections = serde_json::from_str(&contents)?;
    Ok(selections)
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
    let contest = read_contest_data(&args).await?;
    info!("contest.name: {}", contest.name);

    let sample_offers = contest
        .offerings
        .iter()
        .map(|(offer_type, prices)| {
            debug!(
                "sourced {offer_type:?} with {} outcomes, σ={:.3}",
                prices.len(),
                implied_booksum(prices.values())
            );
            let normal = match &offer_type {
                OfferType::HeadToHead(_, _)
                | OfferType::TotalGoals(_, _)
                | OfferType::CorrectScore(_)
                | OfferType::AsianHandicap(_, _)
                | OfferType::DrawNoBet(_)
                | OfferType::SplitHandicap(_, _, _) => 1.0,
                OfferType::AnytimeGoalscorer
                | OfferType::FirstGoalscorer
                | OfferType::PlayerShotsOnTarget(_)
                | OfferType::AnytimeAssist => {
                    let implied_booksum = implied_booksum(prices.values());
                    let expected_overround = 1.0 + prices.len() as f64 * INCREMENTAL_OVERROUND;
                    implied_booksum / expected_overround
                }
            };
            let offer = fit_offer(offer_type.clone(), prices, normal);
            (offer_type.clone(), offer)
        })
        .collect::<FxHashMap<_, _>>();

    let mut model = Model::try_from(model::Config {
        intervals: INTERVALS,
        max_total_goals: MAX_TOTAL_GOALS,
    })?;

    let score_fitter = ScoreFitter::try_from(score_fitter::Config::default())?;
    score_fitter.fit(&mut model, &sample_offers)?;

    let player_goal_fitter = PlayerGoalFitter;
    let fit_result = player_goal_fitter.fit(&mut model, &sample_offers);
    match fit_result {
        Ok(_) => {}
        Err(FitError::MissingOffer(missing_offer)) => {
            warn!("skipped fitting player goals: {missing_offer}");
        }
        Err(err) => {
            panic!("failed fitting player goals: {err}");
        }
    };

    let player_assist_fitter = PlayerAssistFitter;
    let fit_result = player_assist_fitter.fit(&mut model, &sample_offers);
    match fit_result {
        Ok(_) => {}
        Err(FitError::MissingOffer(missing_offer)) => {
            warn!("skipped fitting player assists: {missing_offer}");
        }
        Err(err) => {
            panic!("failed fitting player assists: {err}");
        }
    };

    let stubs = sample_offers
        .iter()
        .filter(|(offer_type, _)| {
            matches!(
                offer_type,
                OfferType::HeadToHead(_, _)
                    | OfferType::TotalGoals(_, _)
                    | OfferType::AsianHandicap(_, _)
                    | OfferType::DrawNoBet(_)
                    | OfferType::SplitHandicap(_, _, _)
                    | OfferType::CorrectScore(_)
                    | OfferType::FirstGoalscorer
                    | OfferType::AnytimeGoalscorer
                    | OfferType::AnytimeAssist
            )
        })
        .filter(|(_, offer)| {
            let has_nil_price = offer.market.prices.contains(&0.0);
            if has_nil_price { warn!("discarding {offer:?}"); }
            !has_nil_price
        })
        .map(|(_, offer)| Stub {
            offer_type: offer.offer_type.clone(),
            outcomes: offer.outcomes.clone(),
            normal: offer.market.fair_booksum(),
            overround: offer.market.overround.clone(),
        })
        .collect::<Vec<_>>();

    let Timed {
        value: cache_stats,
        elapsed,
    } = model.derive(&stubs, &SINGLE_PRICE_BOUNDS)?;
    debug!(
        "derivation took {elapsed:?} for {} offers ({} outcomes), {cache_stats:?}",
        stubs.len(),
        stubs.iter().map(|stub| stub.outcomes.len()).sum::<usize>()
    );

    {
        let table = Table::default().with_rows({
            const PER_ROW: usize = 4;
            let sorted = sort_tuples(model.offers());
            let mut rows = vec![];
            loop {
                let row = sorted
                    .iter()
                    .skip(rows.len() * PER_ROW)
                    .take(PER_ROW)
                    .map(|(_, offer)| {
                        let header = format!(
                            "{:?}\nΣ={:.3}, σ={:.3}, n={}\n",
                            offer.offer_type,
                            offer.market.fair_booksum(),
                            offer.market.offered_booksum(),
                            offer.market.probs.len(),
                        );
                        let nested = print::tabulate_offer(offer);
                        Cell::from(Content::Composite(vec![header.into(), nested.into()]))
                    })
                    .collect::<Vec<_>>();
                if row.is_empty() {
                    break;
                }
                rows.push(Row::new(Styles::default(), row))
            }
            rows
        });
        info!("Derived prices:\n{}", Console::default().render(&table));
    }

    {
        let fitting_errors = model
            .offers
            .values()
            .map(|fitted| {
                let sample = sample_offers.get(&fitted.offer_type).unwrap();
                (
                    &sample.offer_type,
                    FittingErrors {
                        rmse: fit::compute_error(
                            &sample.market.prices,
                            &fitted.market.prices,
                            &ErrorType::SquaredAbsolute,
                        ),
                        rmsre: fit::compute_error(
                            &sample.market.prices,
                            &fitted.market.prices,
                            &ErrorType::SquaredRelative,
                        ),
                    },
                )
            })
            .collect::<Vec<_>>();

        let fitting_errors = print::tabulate_errors(&sort_tuples(fitting_errors));
        let overrounds = print::tabulate_overrounds(
            &sort_tuples(model.offers())
                .iter()
                .map(|(_, offer)| *offer)
                .collect::<Vec<_>>(),
        );
        info!(
            "Fitting errors and overrounds: ({} offers)\n{}",
            model.offers().len(),
            Console::default().render(&tables::merge(&[fitting_errors, overrounds]))
        );
    }

    if let Some(ref selections) = args.selections {
        let selections = load_selections(selections)?;
        let derivation = model.derive_multi(&selections)?;
        let elapsed = derivation.elapsed;
        let derivation = derivation.value;
        // let relatedness =
        //     compute_relatedness_coefficient(&selections, model.offers(), derivation.probability);
        let scaling_exponent = compute_scaling_exponent(derivation.relatedness);
        let scaled_price = scale_price(&derivation.quotation, scaling_exponent);
        info!("selections: {selections:?}, quotation: {:?}, overround: {:.3}, relatedness: {:.3}, redundancies: {:?}, scaling_exponent: {scaling_exponent:?}, scaled_price: {scaled_price:.3}, took: {elapsed:?}",
            derivation.quotation, derivation.quotation.overround(), derivation.relatedness, derivation.redundancies);
        let mut total_fringes = 0;
        let mut unattainable_fringes = 0;
        for (offer, fringe_vec) in derivation.fringes.into_iter().collect::<BTreeMap<_, _>>() {
            info!("\nfringe offer: {offer:?}");
            for fringe in fringe_vec {
                let scaling_exponent = compute_scaling_exponent(fringe.relatedness);
                let scaled_price = scale_price(&fringe.quotation, scaling_exponent);
                total_fringes += 1;
                if fringe.quotation.probability == 0.0 {
                    unattainable_fringes += 1;
                }
                info!(
                    "  {fringe:?}, overround: {:.3}, scaled_price: {scaled_price:.3}",
                    fringe.quotation.overround()
                );
            }
        }
        info!("{total_fringes} fringes derived, {unattainable_fringes} are unattainable");
    }
    Ok(())
}

fn scale_price(quotation: &DerivedPrice, scaling_exponent: f64) -> f64 {
    if quotation.price.is_finite() {
        let scaled_price = quotation.price / quotation.overround().powf(scaling_exponent - 1.0);
        f64::max(
            *MULTI_PRICE_BOUNDS.start(),
            f64::min(scaled_price, *MULTI_PRICE_BOUNDS.end()),
        )
    } else {
        quotation.price
    }
}

fn compute_scaling_exponent(relatedness: f64) -> f64 {
    0.5 * f64::log10(100.0 * relatedness)
}

fn implied_booksum<'a>(prices: impl Iterator<Item = &'a f64>) -> f64 {
    prices.map(|&price| 1.0 / price).sum()
}

fn fit_offer(offer_type: OfferType, map: &HashMap<Outcome, f64>, normal: f64) -> Offer {
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

fn sort_tuples<K: Ord, V>(tuples: impl IntoIterator<Item = (K, V)>) -> Vec<(K, V)> {
    let tuples = tuples.into_iter();
    let mut tuples = tuples.collect::<Vec<_>>();
    tuples.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    tuples
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
