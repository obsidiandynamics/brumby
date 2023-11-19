use std::collections::{BTreeMap, HashMap};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::bail;
use clap::Parser;

use HAlign::Left;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use tracing::{debug, info};
use brumby::entity::{MarketType, OutcomeType, Over, Player, Score, Side};
use brumby::entity::Player::Named;
use brumby::interval::{explore, IntervalConfig, isolate};

use brumby::linear::matrix::Matrix;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby::opt::{hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome};
use brumby::probs::SliceExt;
use brumby::scoregrid;
use brumby::soccer_data::{ContestSummary, download_by_id};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.04..=301.0;
const ZERO_INFLATION: f64 = 0.0;
const INTERVALS: usize = 12;
const MAX_TOTAL_GOALS: u16 = 8;
const ERROR_TYPE: ErrorType = ErrorType::SquaredAbsolute;

type Odds = HashMap<OutcomeType, f64>;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// file to source the contest data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download contest data by ID
    #[clap(short = 'd', long)]
    download: Option<String>,
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

fn verona_vs_leece() -> HashMap<MarketType, Odds> {
    let h2h = HashMap::from([
        (OutcomeType::Win(Side::Home), 2.7),
        (OutcomeType::Draw, 2.87),
        (OutcomeType::Win(Side::Away), 2.87),
    ]);

    let goals_ou =
        HashMap::from([(OutcomeType::Over(2), 2.47), (OutcomeType::Under(3), 1.5)]);

    let correct_score = HashMap::from([
        // home wins
        (OutcomeType::Score(Score::new(1, 0)), 7.0),
        (OutcomeType::Score(Score::new(2, 0)), 12.0),
        (OutcomeType::Score(Score::new(2, 1)), 10.0),
        (OutcomeType::Score(Score::new(3, 0)), 26.0),
        (OutcomeType::Score(Score::new(3, 1)), 23.0),
        (OutcomeType::Score(Score::new(3, 2)), 46.0),
        (OutcomeType::Score(Score::new(4, 0)), 101.0),
        (OutcomeType::Score(Score::new(4, 1)), 81.0),
        (OutcomeType::Score(Score::new(4, 2)), 126.0),
        (OutcomeType::Score(Score::new(4, 3)), 200.0),
        (OutcomeType::Score(Score::new(5, 0)), 200.0),
        (OutcomeType::Score(Score::new(5, 1)), 200.0),
        (OutcomeType::Score(Score::new(5, 2)), 200.0),
        (OutcomeType::Score(Score::new(5, 3)), 200.0),
        (OutcomeType::Score(Score::new(5, 4)), 200.0),
        // draws
        (OutcomeType::Score(Score::new(0, 0)), 6.75),
        (OutcomeType::Score(Score::new(1, 1)), 5.90),
        (OutcomeType::Score(Score::new(2, 2)), 16.00),
        (OutcomeType::Score(Score::new(3, 3)), 101.00),
        (OutcomeType::Score(Score::new(4, 4)), 200.00),
        (OutcomeType::Score(Score::new(5, 5)), 200.00),
        // away wins
        (OutcomeType::Score(Score::new(0, 1)), 7.25),
        (OutcomeType::Score(Score::new(0, 2)), 12.0),
        (OutcomeType::Score(Score::new(1, 2)), 10.5),
        (OutcomeType::Score(Score::new(0, 3)), 31.0),
        (OutcomeType::Score(Score::new(1, 3)), 23.0),
        (OutcomeType::Score(Score::new(2, 3)), 41.0),
        (OutcomeType::Score(Score::new(0, 4)), 101.0),
        (OutcomeType::Score(Score::new(1, 4)), 81.0),
        (OutcomeType::Score(Score::new(2, 4)), 151.0),
        (OutcomeType::Score(Score::new(3, 4)), 200.0),
        (OutcomeType::Score(Score::new(0, 5)), 200.0),
        (OutcomeType::Score(Score::new(1, 5)), 200.0),
        (OutcomeType::Score(Score::new(2, 5)), 200.0),
        (OutcomeType::Score(Score::new(3, 5)), 200.0),
        (OutcomeType::Score(Score::new(4, 5)), 200.0),
    ]);

    HashMap::from([
        (MarketType::HeadToHead, h2h),
        (MarketType::TotalGoalsOverUnder(Over(2)), goals_ou),
        (MarketType::CorrectScore, correct_score),
    ])
}

fn atlanta_vs_sporting_lisbon() -> HashMap<MarketType, Odds> {
    let h2h = HashMap::from([
        (OutcomeType::Win(Side::Home), 2.0),
        (OutcomeType::Draw, 3.5),
        (OutcomeType::Win(Side::Away), 3.4),
    ]);

    let goals_ou =
        HashMap::from([(OutcomeType::Over(2), 1.66), (OutcomeType::Under(3), 2.1)]);

    let correct_score = HashMap::from([
        // home wins
        (OutcomeType::Score(Score::new(1, 0)), 9.25),
        (OutcomeType::Score(Score::new(2, 0)), 11.5),
        (OutcomeType::Score(Score::new(2, 1)), 8.75),
        (OutcomeType::Score(Score::new(3, 0)), 19.0),
        (OutcomeType::Score(Score::new(3, 1)), 15.0),
        (OutcomeType::Score(Score::new(3, 2)), 21.0),
        (OutcomeType::Score(Score::new(4, 0)), 46.0),
        (OutcomeType::Score(Score::new(4, 1)), 34.0),
        (OutcomeType::Score(Score::new(4, 2)), 61.0),
        (OutcomeType::Score(Score::new(4, 3)), 126.0),
        (OutcomeType::Score(Score::new(5, 0)), 126.0),
        (OutcomeType::Score(Score::new(5, 1)), 126.0),
        (OutcomeType::Score(Score::new(5, 2)), 176.0),
        (OutcomeType::Score(Score::new(5, 3)), 200.0),
        (OutcomeType::Score(Score::new(5, 4)), 200.0),
        // draws
        (OutcomeType::Score(Score::new(0, 0)), 13.0),
        (OutcomeType::Score(Score::new(1, 1)), 7.0),
        (OutcomeType::Score(Score::new(2, 2)), 12.5),
        (OutcomeType::Score(Score::new(3, 3)), 51.00),
        (OutcomeType::Score(Score::new(4, 4)), 200.00),
        (OutcomeType::Score(Score::new(5, 5)), 200.00),
        // away wins
        (OutcomeType::Score(Score::new(0, 1)), 14.0),
        (OutcomeType::Score(Score::new(0, 2)), 21.0),
        (OutcomeType::Score(Score::new(1, 2)), 12.5),
        (OutcomeType::Score(Score::new(0, 3)), 41.0),
        (OutcomeType::Score(Score::new(1, 3)), 26.0),
        (OutcomeType::Score(Score::new(2, 3)), 34.0),
        (OutcomeType::Score(Score::new(0, 4)), 126.0),
        (OutcomeType::Score(Score::new(1, 4)), 81.0),
        (OutcomeType::Score(Score::new(2, 4)), 101.0),
        (OutcomeType::Score(Score::new(3, 4)), 151.0),
        (OutcomeType::Score(Score::new(0, 5)), 200.0),
        (OutcomeType::Score(Score::new(1, 5)), 200.0),
        (OutcomeType::Score(Score::new(2, 5)), 200.0),
        (OutcomeType::Score(Score::new(3, 5)), 200.0),
        (OutcomeType::Score(Score::new(4, 5)), 200.0),
    ]);

    let first_goalscorer = HashMap::from([
        (OutcomeType::Player(Named(Side::Home, "Muriel".into())), 5.25),
        (OutcomeType::Player(Named(Side::Home, "Scamacca".into())), 5.8),
        (OutcomeType::Player(Named(Side::Home, "Lookman".into())), 6.25),
        (OutcomeType::Player(Named(Side::Home, "Miranchuk".into())), 7.75),
        (OutcomeType::Player(Named(Side::Home, "Pasalic".into())), 9.0),
        (OutcomeType::Player(Named(Side::Home, "Koopmeiners".into())), 9.25),
        (OutcomeType::Player(Named(Side::Home, "Ederson".into())), 9.5),
        (OutcomeType::Player(Named(Side::Home, "Cisse".into())), 9.5),
        (OutcomeType::Player(Named(Side::Home, "Bakker".into())), 9.75),
        (OutcomeType::Player(Named(Side::Home, "Holm".into())), 11.0),
        (OutcomeType::Player(Named(Side::Home, "Toloi".into())), 16.0),
        (OutcomeType::Player(Named(Side::Home, "Hateboer".into())), 17.0),
        (OutcomeType::Player(Named(Side::Home, "Mendicino".into())), 18.0),
        (OutcomeType::Player(Named(Side::Home, "Scalvini".into())), 21.0),
        (OutcomeType::Player(Named(Side::Home, "Bonfanti".into())), 21.0),
        (OutcomeType::Player(Named(Side::Home, "Adopo".into())), 23.0),
        (OutcomeType::Player(Named(Side::Home, "Zortea".into())), 23.0),
        (OutcomeType::Player(Named(Side::Home, "Kolasinac".into())), 23.0),
        (OutcomeType::Player(Named(Side::Home, "Djimsiti".into())), 26.0),
        (OutcomeType::Player(Named(Side::Home, "De Roon".into())), 26.0),
        (OutcomeType::Player(Named(Side::Home, "Ruggeri".into())), 31.0),
        (OutcomeType::Player(Named(Side::Home, "Del Lungo".into())), 61.0),
        (OutcomeType::Player(Named(Side::Away, "Gyokeres".into())), 6.25),
        (OutcomeType::Player(Named(Side::Away, "Santos".into())), 8.5),
        (OutcomeType::Player(Named(Side::Away, "Paulinho".into())), 8.75),
        (OutcomeType::Player(Named(Side::Away, "Pote".into())), 8.75),
        (OutcomeType::Player(Named(Side::Away, "Edwards".into())), 9.75),
        (OutcomeType::Player(Named(Side::Away, "Ribeiro".into())), 10.5),
        (OutcomeType::Player(Named(Side::Away, "Trincao".into())), 11.0),
        (OutcomeType::Player(Named(Side::Away, "Moreira".into())), 13.0),
        (OutcomeType::Player(Named(Side::Away, "Morita".into())), 15.0),
        (OutcomeType::Player(Named(Side::Away, "Braganca".into())), 21.0),
        (OutcomeType::Player(Named(Side::Away, "Catamo".into())), 29.0),
        (OutcomeType::Player(Named(Side::Away, "Essugo".into())), 31.0),
        (OutcomeType::Player(Named(Side::Away, "Reis".into())), 31.0),
        (OutcomeType::Player(Named(Side::Away, "Esgaio".into())), 31.0),
        (OutcomeType::Player(Named(Side::Away, "St. Juste".into())), 34.0),
        (OutcomeType::Player(Named(Side::Away, "Hjulmand".into())), 34.0),
        (OutcomeType::Player(Named(Side::Away, "Coates".into())), 34.0),
        (OutcomeType::Player(Named(Side::Away, "Diomande".into())), 41.0),
        (OutcomeType::Player(Named(Side::Away, "Quaresma".into())), 51.0),
        (OutcomeType::Player(Named(Side::Away, "Inacio".into())), 51.0),
        (OutcomeType::Player(Named(Side::Away, "Fresneda".into())), 61.0),
        (OutcomeType::Player(Named(Side::Away, "Neto".into())), 71.0),
        (OutcomeType::None, 11.5),
    ]);

    let anytime_goalscorer = HashMap::from([
        (OutcomeType::Player(Named(Side::Home, "Muriel".into())), 2.4),
        (OutcomeType::Player(Named(Side::Home, "Scamacca".into())), 2.7),
        (OutcomeType::Player(Named(Side::Home, "Lookman".into())), 2.85),
        (OutcomeType::Player(Named(Side::Home, "Miranchuk".into())), 3.5),
        (OutcomeType::Player(Named(Side::Home, "Pasalic".into())), 4.0),
        (OutcomeType::Player(Named(Side::Home, "Koopmeiners".into())), 4.25),
        (OutcomeType::Player(Named(Side::Home, "Ederson".into())), 4.2),
        (OutcomeType::Player(Named(Side::Home, "Cisse".into())), 4.25),
        (OutcomeType::Player(Named(Side::Home, "Bakker".into())), 4.4),
        (OutcomeType::Player(Named(Side::Home, "Holm".into())), 4.9),
        (OutcomeType::Player(Named(Side::Home, "Toloi".into())), 8.5),
        (OutcomeType::Player(Named(Side::Home, "Hateboer".into())), 9.0),
        (OutcomeType::Player(Named(Side::Home, "Mendicino".into())), 9.25),
        (OutcomeType::Player(Named(Side::Home, "Scalvini".into())), 10.5),
        (OutcomeType::Player(Named(Side::Home, "Bonfanti".into())), 11.0),
        (OutcomeType::Player(Named(Side::Home, "Adopo".into())), 13.0),
        (OutcomeType::Player(Named(Side::Home, "Zortea".into())), 12.0),
        (OutcomeType::Player(Named(Side::Home, "Kolasinac".into())), 12.5),
        (OutcomeType::Player(Named(Side::Home, "Djimsiti".into())), 13.0),
        (OutcomeType::Player(Named(Side::Home, "De Roon".into())), 14.0),
        (OutcomeType::Player(Named(Side::Home, "Ruggeri".into())), 15.0),
        (OutcomeType::Player(Named(Side::Home, "Del Lungo".into())), 26.0),
        (OutcomeType::Player(Named(Side::Away, "Gyokeres".into())), 2.75),
        (OutcomeType::Player(Named(Side::Away, "Santos".into())), 3.6),
        (OutcomeType::Player(Named(Side::Away, "Paulinho".into())), 3.75),
        (OutcomeType::Player(Named(Side::Away, "Pote".into())), 3.8),
        (OutcomeType::Player(Named(Side::Away, "Edwards".into())), 4.25),
        (OutcomeType::Player(Named(Side::Away, "Ribeiro".into())), 4.5),
        (OutcomeType::Player(Named(Side::Away, "Trincao".into())), 4.8),
        (OutcomeType::Player(Named(Side::Away, "Moreira".into())), 6.25),
        (OutcomeType::Player(Named(Side::Away, "Morita".into())), 7.75),
        (OutcomeType::Player(Named(Side::Away, "Braganca".into())), 11.0),
        (OutcomeType::Player(Named(Side::Away, "Catamo".into())), 14.0),
        (OutcomeType::Player(Named(Side::Away, "Essugo".into())), 15.0),
        (OutcomeType::Player(Named(Side::Away, "Reis".into())), 15.0),
        (OutcomeType::Player(Named(Side::Away, "Esgaio".into())), 16.0),
        (OutcomeType::Player(Named(Side::Away, "St. Juste".into())), 17.0),
        (OutcomeType::Player(Named(Side::Away, "Hjulmand".into())), 7.75),
        (OutcomeType::Player(Named(Side::Away, "Coates".into())), 16.0),
        (OutcomeType::Player(Named(Side::Away, "Diomande".into())), 18.0),
        (OutcomeType::Player(Named(Side::Away, "Quaresma".into())), 21.0),
        (OutcomeType::Player(Named(Side::Away, "Inacio".into())), 26.0),
        (OutcomeType::Player(Named(Side::Away, "Fresneda".into())), 26.0),
        (OutcomeType::Player(Named(Side::Away, "Neto".into())), 31.0),
    ]);

    HashMap::from([
        (MarketType::HeadToHead, h2h),
        (MarketType::TotalGoalsOverUnder(Over(2)), goals_ou),
        (MarketType::CorrectScore, correct_score),
        (MarketType::FirstGoalscorer, first_goalscorer),
        (MarketType::AnytimeGoalscorer, anytime_goalscorer),
    ])
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

    // let ext_markets = atlanta_vs_sporting_lisbon();
    let correct_score_prices = contest.offerings[&MarketType::CorrectScore].clone();
    let h2h_prices = contest.offerings[&MarketType::HeadToHead].clone();
    let goals_ou_prices = contest.offerings[&MarketType::TotalGoalsOverUnder(Over(2))].clone();
    let first_gs = contest.offerings[&MarketType::FirstGoalscorer].clone();
    let anytime_gs = contest.offerings[&MarketType::AnytimeGoalscorer].clone();

    let h2h = fit_market(MarketType::HeadToHead, &h2h_prices);
    // println!("h2h: {h2h:?}");
    let goals_ou = fit_market(MarketType::TotalGoalsOverUnder(Over(2)), &goals_ou_prices);
    // println!("goals_ou: {goals_ou:?}");
    let correct_score = fit_market(MarketType::CorrectScore, &correct_score_prices);
    // println!("correct_score: {correct_score:?}");
    let first_gs = fit_market(MarketType::FirstGoalscorer, &first_gs);
    // println!("first_gs: {first_gs:?}");
    let anytime_gs = fit_market(MarketType::AnytimeGoalscorer, &anytime_gs);
    // println!("anytime_gs: {anytime_gs:?}");

    println!("*** fitting scoregrid ***");
    let start = Instant::now();
    let search_outcome = fit_scoregrid(&[&h2h, &goals_ou]);
    // let search_outcome = fit_scoregrid(&[&correct_score]);
    let elapsed = start.elapsed();
    println!("{elapsed:?} elapsed: search outcome: {search_outcome:?}");

    let scoregrid = interval_scoregrid(
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
        search_outcome.optimal_values[2]
    );
    // println!("scoregrid:\n{}sum: {}", scoregrid.verbose(), scoregrid.flatten().sum());

    let mut fitted_goalscorer_probs = BTreeMap::new();
    for (index, outcome) in first_gs.outcomes.iter().enumerate() {
        match outcome {
            OutcomeType::Player(player) => {
                let player_search_outcome = fit_first_goalscorer(&search_outcome.optimal_values, player, first_gs.market.probs[index]);
                // println!("for player {player:?}, {player_search_outcome:?}");
                fitted_goalscorer_probs.insert(player.clone(), player_search_outcome.optimal_values[0]);
            }
            OutcomeType::None => {},
            _ => unreachable!()
        }
    }

    //TODO need an uninflated draw probability

    //TODO why doesn't the fitted_goalscorer_probs sum to 1?

    let mut fitted_first_goalscorer_probs = vec![];
    for (player, prob) in &fitted_goalscorer_probs {
        let exploration = explore(&IntervalConfig {
            intervals: INTERVALS as u8,
            home_prob: search_outcome.optimal_values[0],
            away_prob: search_outcome.optimal_values[1],
            common_prob: search_outcome.optimal_values[2],
            max_total_goals: MAX_TOTAL_GOALS,
            players: vec![(player.clone(), *prob)],
        });
        let isolated_prob = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(player.clone()), &exploration.prospects, &exploration.player_lookup);
        fitted_first_goalscorer_probs.push(isolated_prob);
        // println!("first scorer {player:?}, prob: {isolated_prob:.3}");
    }
    fitted_first_goalscorer_probs.push(scoregrid[(0, 0)]);
    let anytime_goalscorer_booksum = fitted_first_goalscorer_probs.sum();
    println!("first scorer sum: {anytime_goalscorer_booksum}");
    let fitted_first_goalscorer = LabelledMarket {
        market_type: MarketType::FirstGoalscorer,
        outcomes: first_gs.outcomes.clone(),
        market: Market::frame(&first_gs.market.overround, fitted_first_goalscorer_probs, &SINGLE_PRICE_BOUNDS),
    };
    let table_first_goalscorer = print_market(&fitted_first_goalscorer);
    println!(
        "First Goalscorer:\n{}",
        Console::default().render(&table_first_goalscorer)
    );

    let mut fitted_anytime_goalscorer_outcomes = vec![];
    let mut fitted_anytime_goalscorer_probs = vec![];
    for (player, prob) in &fitted_goalscorer_probs {
        fitted_anytime_goalscorer_outcomes.push(OutcomeType::Player(player.clone()));
        let exploration = explore(&IntervalConfig {
            intervals: INTERVALS as u8,
            home_prob: search_outcome.optimal_values[0],
            away_prob: search_outcome.optimal_values[1],
            common_prob: search_outcome.optimal_values[2],
            max_total_goals: MAX_TOTAL_GOALS,
            players: vec![(player.clone(), *prob)],
        });
        let isolated_prob = isolate(&MarketType::AnytimeGoalscorer, &OutcomeType::Player(player.clone()), &exploration.prospects, &exploration.player_lookup);
        fitted_anytime_goalscorer_probs.push(isolated_prob);
        // println!("anytime scorer {player:?}, prob: {isolated_prob:.3}");
    }
    fitted_anytime_goalscorer_outcomes.push(OutcomeType::None);
    fitted_anytime_goalscorer_probs.push(scoregrid[(0, 0)]);

    // println!("anytime scorer {:?}, prob: {:.3}", OutcomeType::None, scoregrid[(0, 0)]);
    fitted_anytime_goalscorer_probs.scale(1.0 / (1.0 - scoregrid[(0, 0)]));
    let anytime_goalscorer_booksum = fitted_anytime_goalscorer_probs.sum();
    println!("anytime scorer sum: {anytime_goalscorer_booksum}");
    let anytime_goalscorer_overround = Market::fit(&OVERROUND_METHOD, anytime_gs.market.prices.clone(), anytime_goalscorer_booksum);
    let fitted_anytime_goalscorer = LabelledMarket {
        market_type: MarketType::AnytimeGoalscorer,
        outcomes: fitted_anytime_goalscorer_outcomes,
        market: Market::frame(&anytime_goalscorer_overround.overround, fitted_anytime_goalscorer_probs, &SINGLE_PRICE_BOUNDS),
    };
    let table_anytime_goalscorer = print_market(&fitted_anytime_goalscorer);
    println!(
        "Anytime Goalscorer:\n{}",
        Console::default().render(&table_anytime_goalscorer)
    );
    // let draw_prob = scoregrid[(0, 0)];
    // anytime_goalscorer_probs.push(draw_prob);


    let fitted_h2h = frame_prices(&scoregrid, &h2h.outcomes, &h2h.market.overround);
    let fitted_h2h = LabelledMarket {
        market_type: MarketType::HeadToHead,
        outcomes: h2h.outcomes.clone(),
        market: fitted_h2h,
    };
    let table_h2h = print_market(&fitted_h2h);
    println!("H2H:\n{}", Console::default().render(&table_h2h));

    let fitted_goals_ou = frame_prices(&scoregrid, &goals_ou.outcomes, &goals_ou.market.overround);
    let fitted_goals_ou = LabelledMarket {
        market_type: MarketType::TotalGoalsOverUnder(Over(2)),
        outcomes: goals_ou.outcomes.clone(),
        market: fitted_goals_ou,
    };
    let table_goals_ou = print_market(&fitted_goals_ou);
    println!("Goals O/U:\n{}", Console::default().render(&table_goals_ou));

    let fitted_correct_score = frame_prices(
        &scoregrid,
        &correct_score.outcomes,
        &correct_score.market.overround,
    );
    let fitted_correct_score = LabelledMarket {
        market_type: MarketType::CorrectScore,
        outcomes: correct_score.outcomes.clone(),
        market: fitted_correct_score,
    };
    let table_correct_score = print_market(&fitted_correct_score);
    println!(
        "Correct Score:\n{}",
        Console::default().render(&table_correct_score)
    );

    let market_errors = [
        ("H2H", &h2h, &fitted_h2h),
        ("Goals O/U", &goals_ou, &fitted_goals_ou),
        ("Correct Score", &correct_score, &fitted_correct_score),
        ("First Goalscorer", &first_gs, &fitted_first_goalscorer),
        ("Anytime Goalscorer", &anytime_gs, &fitted_anytime_goalscorer),
    ]
    .iter()
    .map(|(key, sample, fitted)| {
        (
            *key,
            compute_error(&sample.market.prices, &fitted.market.prices),
        )
    })
    .collect::<Vec<_>>();
    let table_errors = print_errors(&market_errors);
    println!(
        "Fitting errors:\n{}",
        Console::default().render(&table_errors)
    );

    let table_overrounds = print_overrounds(&[fitted_h2h, fitted_goals_ou, fitted_correct_score, fitted_first_goalscorer, fitted_anytime_goalscorer]);
    println!(
        "Market overrounds:\n{}",
        Console::default().render(&table_overrounds)
    );

    Ok(())
}

fn fit_market(market_type: MarketType, map: &HashMap<OutcomeType, f64>) -> LabelledMarket {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let outcomes = entries
        .iter()
        .map(|(outcome, _)| (*outcome).clone())
        .collect::<Vec<_>>();
    let prices = entries.iter().map(|(_, &price)| price).collect();
    let market = Market::fit(&OVERROUND_METHOD, prices, 1.0);
    LabelledMarket { market_type, outcomes, market }
}

#[derive(Debug)]
pub struct LabelledMarket {
    market_type: MarketType,
    outcomes: Vec<OutcomeType>,
    market: Market,
}

fn fit_scoregrid(markets: &[&LabelledMarket]) -> HypergridSearchOutcome {
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-6,
            bounds: vec![0.0001..=0.5, 0.0001..=0.5, 0.00..=0.1].into(),
            // bounds: Capture::Owned(vec![0.5..=3.5, 0.5..=3.5]),
            // bounds: Capture::Owned(vec![0.5..=3.5, 0.5..=3.5, 0.0..=0.1]), // for the bivariate
            resolution: 4,
        },
        |values| values.sum() <= 1.0,
        |values| {
            // let scoregrid = bivariate_binomial_scoregrid(values[0], values[1], values[2]);
            let scoregrid = interval_scoregrid(values[0], values[1], values[2]);
            let mut residual = 0.0;
            for market in markets {
                for (index, outcome) in market.outcomes.iter().enumerate() {
                    let fitted_prob = outcome.gather(&scoregrid);
                    let sample_prob = market.market.probs[index];
                    residual += ERROR_TYPE.calculate(sample_prob, fitted_prob);
                }
            }
            residual
        },
    )
}

fn fit_first_goalscorer(optimal_scoring_probs: &[f64], player: &Player, expected_prob: f64) -> HypergridSearchOutcome {
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-6,
            bounds: vec![0.0001..=0.2].into(),
            resolution: 4,
        },
        |_| true,
        |values| {
            let exploration = explore(&IntervalConfig {
                intervals: INTERVALS as u8,
                home_prob: optimal_scoring_probs[0],
                away_prob: optimal_scoring_probs[1],
                common_prob: optimal_scoring_probs[2],
                max_total_goals: MAX_TOTAL_GOALS,
                players: vec![(player.clone(), values[0])],
            });
            let isolated_prob = isolate(&MarketType::FirstGoalscorer, &OutcomeType::Player(player.clone()), &exploration.prospects, &exploration.player_lookup);
            ERROR_TYPE.calculate(expected_prob, isolated_prob)
        },
    )
}

enum ErrorType {
    SquaredRelative,
    SquaredAbsolute
}
impl ErrorType {
    fn calculate(&self, expected: f64, sample: f64) -> f64 {
        match self {
            ErrorType::SquaredRelative => ((expected - sample)/sample).powi(2),
            ErrorType::SquaredAbsolute => (expected - sample).powi(2)
        }
    }

    fn reverse(&self, error: f64) -> f64 {
        error.sqrt()
    }
}

/// Intervals.
fn interval_scoregrid(interval_home_prob: f64, interval_away_prob: f64, interval_common_prob: f64) -> Matrix<f64> {
    let dim = usize::min(MAX_TOTAL_GOALS as usize, INTERVALS) + 1;
    let mut scoregrid = Matrix::allocate(dim, dim);
    scoregrid::from_interval(INTERVALS as u8, MAX_TOTAL_GOALS, interval_home_prob, interval_away_prob, interval_common_prob, &mut scoregrid);
    scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
    scoregrid
}

/// Binomial.
fn binomial_scoregrid(interval_home_prob: f64, interval_away_prob: f64) -> Matrix<f64> {
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_binomial(interval_home_prob, interval_away_prob, &mut scoregrid);
    scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
    scoregrid
}

/// Bivariate binomial.
fn bivariate_binomial_scoregrid(interval_home_prob: f64, interval_away_prob: f64, interval_common_prob: f64) -> Matrix<f64> {
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_bivariate_binomial(interval_home_prob, interval_away_prob, interval_common_prob, &mut scoregrid);
    scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
    scoregrid
}

/// Independent Poisson.
fn univariate_poisson_scoregrid(home_rate: f64, away_rate: f64) -> Matrix<f64> {
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_univariate_poisson(home_rate, away_rate, &mut scoregrid);
    scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
    scoregrid
}

/// Bivariate Poisson.
fn bivariate_poisson_scoregrid(home_rate: f64, away_rate: f64, common: f64) -> Matrix<f64> {
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_bivariate_poisson(home_rate, away_rate, common, &mut scoregrid);
    scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
    scoregrid
}

fn frame_prices(scoregrid: &Matrix<f64>, outcomes: &[OutcomeType], overround: &Overround) -> Market {
    let probs = outcomes
        .iter()
        .map(|outcome| outcome.gather(scoregrid))
        .map(|prob| f64::max(0.0001, prob))
        .collect();
    Market::frame(overround, probs, &SINGLE_PRICE_BOUNDS)
}

fn compute_error(sample_prices: &[f64], fitted_prices: &[f64]) -> f64 {
    let mut error_sum = 0.0;
    let mut counted = 0;
    for (index, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[index];
        if fitted_price.is_finite() {
            counted += 1;
            let (sample_prob, fitted_prob) = (1.0 / sample_price, 1.0 / fitted_price);
            error_sum += ERROR_TYPE.calculate(sample_prob, fitted_prob);
        }
    }
    let mean_error = error_sum / counted as f64;
    ERROR_TYPE.reverse(mean_error)
}

fn print_market(market: &LabelledMarket) -> Table {
    let mut table = Table::default().with_cols(vec![
        Col::new(Styles::default().with(MinWidth(10)).with(Left)),
        Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Right)),
    ]);
    for (index, outcome) in market.outcomes.iter().enumerate() {
        table.push_row(Row::new(
            Styles::default(),
            vec![
                format!("{outcome:?}").into(),
                format!("{:.2}", market.market.prices[index]).into(),
            ],
        ));
    }
    table
}

fn print_errors(errors: &[(&str, f64)]) -> Table {
    let mut table = Table::default().with_cols(vec![
        Col::new(Styles::default().with(MinWidth(10)).with(Left)),
        Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
    ]);
    for (key, error) in errors {
        table.push_row(Row::new(
            Styles::default(),
            vec![key.to_string().into(), format!("{error:.3}").into()],
        ));
    }
    table
}

fn print_overrounds(markets: &[LabelledMarket]) -> Table {
    let mut table = Table::default().with_cols(vec![
        Col::new(Styles::default().with(MinWidth(10)).with(Left)),
        Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
    ]);
    for market in markets {
        table.push_row(Row::new(
            Styles::default(),
            vec![format!("{:?}", market.market_type).into(), format!("{:.3}", market.market.overround.value).into()],
        ));
    }
    table
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