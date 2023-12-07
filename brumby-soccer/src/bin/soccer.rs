use std::collections::{BTreeMap, HashMap};
use std::env;
use std::error::Error;
use std::ops::Range;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::bail;
use clap::Parser;
use HAlign::Left;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use tracing::{debug, info};

use brumby::{factorial, poisson};
use brumby_soccer::{scoregrid};
use brumby_soccer::domain::{MarketType, OutcomeType, Over, Period, Player, Side};
use brumby_soccer::domain::Player::Named;
use brumby_soccer::interval::{explore, IntervalConfig, isolate, ScoringProbs};
use brumby::linear::matrix::Matrix;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby::opt::{
    hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome, univariate_descent,
    UnivariateDescentConfig, UnivariateDescentOutcome,
};
use brumby::probs::SliceExt;
use brumby_soccer::scoregrid::{from_correct_score, home_away_expectations};
use brumby_soccer::soccer_data::{ContestSummary, download_by_id};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.01..=301.0;
const FIRST_GOALSCORER_BOOKSUM: f64 = 1.0;
const INTERVALS: usize = 18;
const MAX_TOTAL_GOALS_HALF: u16 = 4;
const MAX_TOTAL_GOALS_FULL: u16 = 8;
const ERROR_TYPE: ErrorType = ErrorType::SquaredRelative;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// file to source the contest data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download contest data by ID
    #[clap(short = 'd', long)]
    download: Option<String>,

    /// print player markets
    #[clap(short = 'p', long = "players")]
    print_players: bool,
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
    let contest = read_contest_data(&args).await?;
    info!("contest.name: {}", contest.name);

    let ft_correct_score_prices =
        contest.offerings[&MarketType::CorrectScore(Period::FullTime)].clone();
    let ft_h2h_prices = contest.offerings[&MarketType::HeadToHead(Period::FullTime)].clone();
    let ft_goals_ou_prices =
        contest.offerings[&MarketType::TotalGoalsOverUnder(Period::FullTime, Over(2))].clone();
    let h1_h2h_prices = contest.offerings[&MarketType::HeadToHead(Period::FirstHalf)].clone();
    let h1_goals_ou_prices =
        contest.offerings[&MarketType::TotalGoalsOverUnder(Period::FirstHalf, Over(2))].clone();
    let h2_h2h_prices = contest.offerings[&MarketType::HeadToHead(Period::SecondHalf)].clone();
    let h2_goals_ou_prices =
        contest.offerings[&MarketType::TotalGoalsOverUnder(Period::SecondHalf, Over(2))].clone();
    let first_gs = contest.offerings[&MarketType::FirstGoalscorer].clone();
    let anytime_gs = contest.offerings[&MarketType::AnytimeGoalscorer].clone();

    let ft_h2h = fit_market(
        MarketType::HeadToHead(Period::FullTime),
        &ft_h2h_prices,
        1.0,
    );
    let ft_goals_ou = fit_market(
        MarketType::TotalGoalsOverUnder(Period::FullTime, Over(2)),
        &ft_goals_ou_prices,
        1.0,
    );
    let ft_correct_score = fit_market(
        MarketType::CorrectScore(Period::FullTime),
        &ft_correct_score_prices,
        1.0,
    );
    let h1_h2h = fit_market(
        MarketType::HeadToHead(Period::FirstHalf),
        &h1_h2h_prices,
        1.0,
    );
    let h1_goals_ou = fit_market(
        MarketType::TotalGoalsOverUnder(Period::FirstHalf, Over(2)),
        &h1_goals_ou_prices,
        1.0,
    );
    let h2_h2h = fit_market(
        MarketType::HeadToHead(Period::SecondHalf),
        &h2_h2h_prices,
        1.0,
    );
    let h2_goals_ou = fit_market(
        MarketType::TotalGoalsOverUnder(Period::SecondHalf, Over(2)),
        &h2_goals_ou_prices,
        1.0,
    );

    // first half
    println!("*** fitting H1 ***");
    let h1_search_outcome = fit_scoregrid_half(&[&h1_h2h, &h1_goals_ou]);

    // second half
    println!("*** fitting H2 ***");
    let h2_search_outcome = fit_scoregrid_half(&[&h2_h2h, &h2_goals_ou]);

    let ft_search_outcome = {
        let init_estimates = {
            println!("*** F/T: fitting bivariate poisson scoregrid ***");
            let start = Instant::now();
            let search_outcome = fit_bivariate_poisson_scoregrid(&[&ft_h2h, &ft_goals_ou], MAX_TOTAL_GOALS_FULL);
            let elapsed = start.elapsed();
            println!("F/T: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_lambdas(&search_outcome.optimal_values));
            search_outcome
                .optimal_values
                .iter()
                .map(|optimal_value| {
                    1.0 - poisson::univariate(
                        0,
                        optimal_value / INTERVALS as f64,
                        &factorial::Calculator,
                    )
                })
                .collect::<Vec<_>>()
        };
        println!("F/T: initial estimates: {init_estimates:?}");

        println!("*** F/T: fitting bivariate binomial scoregrid ***");
        let start = Instant::now();
        let search_outcome =
            fit_bivariate_binomial_scoregrid(&[&ft_h2h, &ft_goals_ou], &init_estimates, INTERVALS as u8, MAX_TOTAL_GOALS_FULL);
        // let search_outcome = fit_scoregrid(&[&correct_score]);
        let elapsed = start.elapsed();
        println!("F/T: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
        search_outcome
    };

    let mut adj_optimal_h1 = [0.0; 3];
    let mut adj_optimal_h2 = [0.0; 3];
    for (i, orig_h1) in h1_search_outcome.optimal_values.iter().enumerate() {
        let orig_h2 = h2_search_outcome.optimal_values[i];
        let ft = ft_search_outcome.optimal_values[i];
        let avg_h1_h2 = (orig_h1 + orig_h2) / 2.0;
        adj_optimal_h1[i] = orig_h1 / (avg_h1_h2 / ft);
        adj_optimal_h2[i] = orig_h2 / (avg_h1_h2 / ft);
    }
    println!("adjusted optimal_h1={adj_optimal_h1:?}, optimal_h2={adj_optimal_h2:?}");
    // let optimal_h1 = h1_search_outcome.optimal_values;
    // let optimal_h2 = h2_search_outcome.optimal_values;

    // let ft_gamma_sum = ft_search_outcome.optimal_values.sum();
    // h1_search_outcome.optimal_values.normalise(ft_gamma_sum * 1.0);
    // h2_search_outcome.optimal_values.normalise(ft_gamma_sum * 1.0);

    let mut ft_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_FULL);
    // interval_scoregrid(
    //     0..INTERVALS as u8,
    //     ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //     ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //     &mut ft_scoregrid,
    // );
    interval_scoregrid(
        0..INTERVALS as u8,
        MAX_TOTAL_GOALS_FULL,
        ScoringProbs::from(adj_optimal_h1.as_slice()),
        ScoringProbs::from(adj_optimal_h2.as_slice()),
        &mut ft_scoregrid,
    );
    // correct_score_scoregrid(&ft_correct_score, &mut ft_scoregrid);


    let mut h1_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_HALF);
    interval_scoregrid(
        0..(INTERVALS / 2) as u8,
        MAX_TOTAL_GOALS_HALF,
        ScoringProbs::from(adj_optimal_h1.as_slice()),
        ScoringProbs { home_prob: 0.0, away_prob: 0.0, common_prob: 0.0 },
        &mut h1_scoregrid,
    );

    let fitted_h1_h2h = frame_prices(&h1_scoregrid, &ft_h2h.outcomes, &ft_h2h.market.overround);
    let fitted_h1_h2h = LabelledMarket {
        market_type: MarketType::HeadToHead(Period::FirstHalf),
        outcomes: h1_h2h.outcomes.clone(),
        market: fitted_h1_h2h,
    };
    let h1_h2h_table = print_market(&fitted_h1_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h1_h2h.market_type,
        fitted_h1_h2h.market.probs.sum(),
        Console::default().render(&h1_h2h_table)
    );

    let fitted_h1_goals_ou = frame_prices(
        &h1_scoregrid,
        &ft_goals_ou.outcomes,
        &ft_goals_ou.market.overround,
    );
    let fitted_h1_goals_ou = LabelledMarket {
        market_type: MarketType::TotalGoalsOverUnder(Period::FirstHalf, Over(2)),
        outcomes: h1_goals_ou.outcomes.clone(),
        market: fitted_h1_goals_ou,
    };
    let h1_goals_ou_table = print_market(&fitted_h1_goals_ou);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h1_goals_ou.market_type,
        fitted_h1_goals_ou.market.probs.sum(),
        Console::default().render(&h1_goals_ou_table)
    );

    let mut h2_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_HALF);
    // interval_scoregrid(
    //     0..(INTERVALS / 2) as u8,
    //     ModelParams { home_prob: h2_search_outcome.optimal_values[0], away_prob: h2_search_outcome.optimal_values[1], common_prob: h2_search_outcome.optimal_values[2] },
    //     ModelParams { home_prob: 0.0, away_prob: 0.0, common_prob: 0.0 },
    //     &mut h2_scoregrid,
    // );
    interval_scoregrid(
        (INTERVALS / 2) as u8..INTERVALS as u8,
        MAX_TOTAL_GOALS_HALF,
        ScoringProbs { home_prob: 0.0, away_prob: 0.0, common_prob: 0.0 },
        ScoringProbs::from(adj_optimal_h2.as_slice()),
        &mut h2_scoregrid,
    );

    let fitted_h2_h2h = frame_prices(&h2_scoregrid, &h2_h2h.outcomes, &h2_h2h.market.overround);
    let fitted_h2_h2h = LabelledMarket {
        market_type: MarketType::HeadToHead(Period::SecondHalf),
        outcomes: h2_h2h.outcomes.clone(),
        market: fitted_h2_h2h,
    };
    let h2_h2h_table = print_market(&fitted_h2_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h2_h2h.market_type,
        fitted_h2_h2h.market.probs.sum(),
        Console::default().render(&h2_h2h_table)
    );

    let fitted_h2_goals_ou = frame_prices(
        &h2_scoregrid,
        &h2_goals_ou.outcomes,
        &h2_goals_ou.market.overround,
    );
    let fitted_h2_goals_ou = LabelledMarket {
        market_type: MarketType::TotalGoalsOverUnder(Period::SecondHalf, Over(2)),
        outcomes: h2_goals_ou.outcomes.clone(),
        market: fitted_h2_goals_ou,
    };
    let h2_goals_ou_table = print_market(&fitted_h2_goals_ou);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h2_goals_ou.market_type,
        fitted_h2_goals_ou.market.probs.sum(),
        Console::default().render(&h2_goals_ou_table)
    );

    let fitted_ft_h2h = frame_prices(&ft_scoregrid, &ft_h2h.outcomes, &ft_h2h.market.overround);
    let fitted_ft_h2h = LabelledMarket {
        market_type: MarketType::HeadToHead(Period::FullTime),
        outcomes: ft_h2h.outcomes.clone(),
        market: fitted_ft_h2h,
    };
    let ft_h2h_table = print_market(&fitted_ft_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_h2h.market_type,
        fitted_ft_h2h.market.probs.sum(),
        Console::default().render(&ft_h2h_table)
    );

    let fitted_ft_goals_ou = frame_prices(
        &ft_scoregrid,
        &ft_goals_ou.outcomes,
        &ft_goals_ou.market.overround,
    );
    let fitted_ft_goals_ou = LabelledMarket {
        market_type: MarketType::TotalGoalsOverUnder(Period::FullTime, Over(2)),
        outcomes: ft_goals_ou.outcomes.clone(),
        market: fitted_ft_goals_ou,
    };
    let ft_goals_ou_table = print_market(&fitted_ft_goals_ou);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_goals_ou.market_type,
        fitted_ft_goals_ou.market.probs.sum(),
        Console::default().render(&ft_goals_ou_table)
    );

    let fitted_ft_correct_score = frame_prices(
        &ft_scoregrid,
        &ft_correct_score.outcomes,
        &ft_correct_score.market.overround,
    );
    let fitted_ft_correct_score = LabelledMarket {
        market_type: MarketType::CorrectScore(Period::FullTime),
        outcomes: ft_correct_score.outcomes.clone(),
        market: fitted_ft_correct_score,
    };
    let ft_correct_score_table = print_market(&fitted_ft_correct_score);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_correct_score.market_type,
        fitted_ft_correct_score.market.probs.sum(),
        Console::default().render(&ft_correct_score_table),
    );

    // let mut future_scoregrid = allocate_scoregrid();
    // correct_score_scoregrid(&ft_correct_score, &mut future_scoregrid);
    // let h2_scoregrid = subtract(&ft_scoregrid, &h1_scoregrid);
    // println!("ft_scoregrid.sum={}", ft_scoregrid.flatten().sum());
    // println!("h1_scoregrid.sum={}", h1_scoregrid.flatten().sum());
    // println!("h2_scoregrid.sum={}", h2_scoregrid.flatten().sum());


    let home_away_expectations = home_away_expectations(&ft_scoregrid);
    println!(
        "p(0, 0)={}, home + away expectations: ({} + {} = {})",
        ft_scoregrid[(0, 0)],
        home_away_expectations.0,
        home_away_expectations.1,
        home_away_expectations.0 + home_away_expectations.1
    );

    let first_gs = fit_market(
        MarketType::FirstGoalscorer,
        &first_gs,
        FIRST_GOALSCORER_BOOKSUM,
    );
    let anytime_gs = fit_market(MarketType::AnytimeGoalscorer, &anytime_gs, 1.0);

    // println!("scoregrid:\n{}sum: {}", scoregrid.verbose(), scoregrid.flatten().sum());
    let home_ratio = (ft_search_outcome.optimal_values[0] + ft_search_outcome.optimal_values[2] / 2.0)
        / ft_search_outcome.optimal_values.sum()
        * (1.0 - ft_scoregrid[(0, 0)]);
    let away_ratio = (ft_search_outcome.optimal_values[1] + ft_search_outcome.optimal_values[2] / 2.0)
        / ft_search_outcome.optimal_values.sum()
        * (1.0 - ft_scoregrid[(0, 0)]);
    // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    let mut fitted_goalscorer_probs = BTreeMap::new();
    let start = Instant::now();
    for (index, outcome) in first_gs.outcomes.iter().enumerate() {
        match outcome {
            OutcomeType::Player(player) => {
                let side_ratio = match player {
                    Named(side, _) => match side {
                        Side::Home => home_ratio,
                        Side::Away => away_ratio,
                    },
                    Player::Other => unreachable!(),
                };
                let init_estimate = first_gs.market.probs[index] / side_ratio;
                let player_search_outcome = fit_first_goalscorer(
                    // &ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                    //  &ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                    &ScoringProbs { home_prob: adj_optimal_h1[0], away_prob: adj_optimal_h1[1], common_prob: adj_optimal_h1[2] },
                    &ScoringProbs { home_prob: adj_optimal_h2[0], away_prob: adj_optimal_h2[1], common_prob: adj_optimal_h2[2] },
                    player,
                    init_estimate,
                    first_gs.market.probs[index],
                );
                // println!("for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", first_gs.market.probs[index]);
                fitted_goalscorer_probs.insert(player.clone(), player_search_outcome.optimal_value);
            }
            OutcomeType::None => {}
            _ => unreachable!(),
        }
    }
    let elapsed = start.elapsed();
    println!("player fitting took {elapsed:?}");

    let mut fitted_first_goalscorer_probs = vec![];
    for (player, prob) in &fitted_goalscorer_probs {
        let exploration = explore(
            &IntervalConfig {
                intervals: INTERVALS as u8,
                // h1_probs: ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                // h2_probs: ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                h1_probs: ScoringProbs { home_prob: adj_optimal_h1[0], away_prob: adj_optimal_h1[1], common_prob: adj_optimal_h1[2] },
                h2_probs: ScoringProbs { home_prob: adj_optimal_h2[0], away_prob: adj_optimal_h2[1], common_prob: adj_optimal_h2[2] },
                max_total_goals: MAX_TOTAL_GOALS_FULL,
                players: vec![(player.clone(), *prob)],
            },
            0..INTERVALS as u8,
        );
        let isolated_prob = isolate(
            &MarketType::FirstGoalscorer,
            &OutcomeType::Player(player.clone()),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        fitted_first_goalscorer_probs.push(isolated_prob);
        // println!("first scorer {player:?}, prob: {isolated_prob:.3}");
    }
    fitted_first_goalscorer_probs.push(ft_scoregrid[(0, 0)]);
    fitted_first_goalscorer_probs.normalise(FIRST_GOALSCORER_BOOKSUM);
    // fitted_first_goalscorer_probs.push(1.0 - fitted_first_goalscorer_probs.sum());

    let fitted_first_goalscorer = LabelledMarket {
        market_type: MarketType::FirstGoalscorer,
        outcomes: first_gs.outcomes.clone(),
        market: Market::frame(
            &first_gs.market.overround,
            fitted_first_goalscorer_probs,
            &SINGLE_PRICE_BOUNDS,
        ),
    };

    if args.print_players {
        println!(
            "sample first goalscorer σ={:.3}",
            implied_booksum(&first_gs.market.prices)
        );
        let table_first_goalscorer = print_market(&fitted_first_goalscorer);
        println!(
            "{:?}: [Σ={:.3}, σ={:.3}, n={}]\n{}",
            fitted_first_goalscorer.market_type,
            fitted_first_goalscorer.market.probs.sum(),
            implied_booksum(&fitted_first_goalscorer.market.prices),
            fitted_first_goalscorer.market.probs.len(),
            Console::default().render(&table_first_goalscorer)
        );
    }

    let mut fitted_anytime_goalscorer_outcomes = vec![];
    let mut fitted_anytime_goalscorer_probs = vec![];
    for (player, prob) in &fitted_goalscorer_probs {
        fitted_anytime_goalscorer_outcomes.push(OutcomeType::Player(player.clone()));
        let exploration = explore(
            &IntervalConfig {
                intervals: INTERVALS as u8,
                // h1_probs: ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                // h2_probs: ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
                h1_probs: ScoringProbs::from(adj_optimal_h1.as_slice()),
                h2_probs: ScoringProbs::from(adj_optimal_h2.as_slice()),
                max_total_goals: MAX_TOTAL_GOALS_FULL,
                players: vec![(player.clone(), *prob)],
            },
            0..INTERVALS as u8,
        );
        let isolated_prob = isolate(
            &MarketType::AnytimeGoalscorer,
            &OutcomeType::Player(player.clone()),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        fitted_anytime_goalscorer_probs.push(isolated_prob);
        // println!("anytime scorer {player:?}, prob: {isolated_prob:.3}");
    }
    fitted_anytime_goalscorer_outcomes.push(OutcomeType::None);
    // fitted_anytime_goalscorer_probs.normalise(home_away_expectations.0 + home_away_expectations.1);
    fitted_anytime_goalscorer_probs.push(ft_scoregrid[(0, 0)]);

    let anytime_goalscorer_booksum = fitted_anytime_goalscorer_probs.sum();
    let anytime_goalscorer_overround = Market::fit(
        &OVERROUND_METHOD,
        anytime_gs.market.prices.clone(),
        anytime_goalscorer_booksum,
    );
    let fitted_anytime_goalscorer = LabelledMarket {
        market_type: MarketType::AnytimeGoalscorer,
        outcomes: fitted_anytime_goalscorer_outcomes,
        market: Market::frame(
            &anytime_goalscorer_overround.overround,
            fitted_anytime_goalscorer_probs,
            &SINGLE_PRICE_BOUNDS,
        ),
    };

    if args.print_players {
        println!(
            "sample anytime goalscorer σ={:.3}",
            implied_booksum(&anytime_gs.market.prices)
        );
        let table_anytime_goalscorer = print_market(&fitted_anytime_goalscorer);
        println!(
            "{:?}: [Σ={:.3}, σ={:.3}, n={}]\n{}",
            fitted_anytime_goalscorer.market_type,
            fitted_anytime_goalscorer.market.probs.sum(),
            implied_booksum(&fitted_anytime_goalscorer.market.prices),
            fitted_first_goalscorer.market.probs.len(),
            Console::default().render(&table_anytime_goalscorer)
        );
    }

    let market_errors = [
        (&h1_h2h, &fitted_h1_h2h),
        (&h1_goals_ou, &fitted_h1_goals_ou),
        (&h2_h2h, &fitted_h2_h2h),
        (&h2_goals_ou, &fitted_h2_goals_ou),
        (&ft_h2h, &fitted_ft_h2h),
        (&ft_goals_ou, &fitted_ft_goals_ou),
        (&ft_correct_score, &fitted_ft_correct_score),
        (&first_gs, &fitted_first_goalscorer),
        (&anytime_gs, &fitted_anytime_goalscorer),
    ]
    .iter()
    .map(|(sample, fitted)| {
        (
            &sample.market_type,
            MarketErrors {
                rmse: compute_error(
                    &sample.market.prices,
                    &fitted.market.prices,
                    &ErrorType::SquaredAbsolute,
                ),
                rmsre: compute_error(
                    &sample.market.prices,
                    &fitted.market.prices,
                    &ErrorType::SquaredRelative,
                ),
            },
        )
    })
    .collect::<Vec<_>>();
    let table_errors = print_errors(&market_errors);
    println!(
        "Fitting errors:\n{}",
        Console::default().render(&table_errors)
    );

    let table_overrounds = print_overrounds(&[
        fitted_h1_h2h,
        fitted_h1_goals_ou,
        fitted_h2_h2h,
        fitted_h2_goals_ou,
        fitted_ft_h2h,
        fitted_ft_goals_ou,
        fitted_ft_correct_score,
        fitted_first_goalscorer,
        fitted_anytime_goalscorer,
    ]);
    println!(
        "Market overrounds:\n{}",
        Console::default().render(&table_overrounds)
    );

    Ok(())
}

fn implied_booksum(prices: &[f64]) -> f64 {
    prices.invert().sum()
}

fn fit_market(
    market_type: MarketType,
    map: &HashMap<OutcomeType, f64>,
    normal: f64,
) -> LabelledMarket {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let outcomes = entries
        .iter()
        .map(|(outcome, _)| (*outcome).clone())
        .collect::<Vec<_>>();
    let prices = entries.iter().map(|(_, &price)| price).collect();
    let market = Market::fit(&OVERROUND_METHOD, prices, normal);
    LabelledMarket {
        market_type,
        outcomes,
        market,
    }
}

#[derive(Debug)]
pub struct LabelledMarket {
    market_type: MarketType,
    outcomes: Vec<OutcomeType>,
    market: Market,
}

fn fit_scoregrid_half(markets: &[&LabelledMarket]) -> HypergridSearchOutcome {
    let init_estimates = {
        let start = Instant::now();
        let search_outcome = fit_bivariate_poisson_scoregrid(markets, MAX_TOTAL_GOALS_HALF);
        let elapsed = start.elapsed();
        println!("biv-poisson: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_lambdas(&search_outcome.optimal_values));
        search_outcome
            .optimal_values
            .iter()
            .map(|optimal_value| {
                1.0 - poisson::univariate(
                    0,
                    optimal_value / INTERVALS as f64 * 2.0,
                    &factorial::Calculator,
                )
            })
            .collect::<Vec<_>>()
    };
    println!("initial estimates: {init_estimates:?}");

    let start = Instant::now();
    let search_outcome =
        fit_bivariate_binomial_scoregrid(markets, &init_estimates, (INTERVALS / 2) as u8, MAX_TOTAL_GOALS_HALF);
    let elapsed = start.elapsed();
    println!("biv-binomial: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
    search_outcome
}

fn fit_bivariate_binomial_scoregrid(
    markets: &[&LabelledMarket],
    init_estimates: &[f64],
    intervals: u8,
    max_total_goals: u16
) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(max_total_goals);
    let bounds = init_estimates
        .iter()
        .map(|&estimate| (estimate * 0.67)..=(estimate * 1.5))
        .collect::<Vec<_>>();
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: bounds.into(),
            resolution: 10,
        },
        |values| values.sum() <= 1.0,
        |values| {
            bivariate_binomial_scoregrid(
                intervals,
                values[0],
                values[1],
                values[2],
                &mut scoregrid,
            );
            scoregrid_error(markets, &scoregrid)
        },
    )
}

fn fit_bivariate_poisson_scoregrid(markets: &[&LabelledMarket], max_total_goals: u16) -> HypergridSearchOutcome {
    let mut scoregrid = allocate_scoregrid(max_total_goals);
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 10,
            acceptable_residual: 1e-6,
            bounds: vec![0.2..=3.0, 0.2..=3.0, 0.0..=0.5].into(),
            resolution: 10,
        },
        |_| true,
        |values| {
            bivariate_poisson_scoregrid(values[0], values[1], values[2], &mut scoregrid);
            scoregrid_error(markets, &scoregrid)
        },
    )
}

fn scoregrid_error(markets: &[&LabelledMarket], scoregrid: &Matrix<f64>) -> f64 {
    let mut residual = 0.0;
    for market in markets {
        for (index, outcome) in market.outcomes.iter().enumerate() {
            let fitted_prob = outcome.gather(scoregrid);
            let sample_prob = market.market.probs[index];
            residual += ERROR_TYPE.calculate(sample_prob, fitted_prob);
        }
    }
    residual
}

fn fit_first_goalscorer(
    h1_probs: &ScoringProbs,
    h2_probs: &ScoringProbs,
    player: &Player,
    init_estimate: f64,
    expected_prob: f64,
) -> UnivariateDescentOutcome {
    univariate_descent(
        &UnivariateDescentConfig {
            init_value: init_estimate,
            init_step: init_estimate * 0.1,
            min_step: init_estimate * 0.001,
            max_steps: 100,
            acceptable_residual: 1e-9,
        },
        |value| {
            let exploration = explore(
                &IntervalConfig {
                    intervals: INTERVALS as u8,
                    h1_probs: h1_probs.clone(),
                    h2_probs: h2_probs.clone(),
                    max_total_goals: MAX_TOTAL_GOALS_FULL,
                    players: vec![(player.clone(), value)],
                },
                0..INTERVALS as u8,
            );
            let isolated_prob = isolate(
                &MarketType::FirstGoalscorer,
                &OutcomeType::Player(player.clone()),
                &exploration.prospects,
                &exploration.player_lookup,
            );
            ERROR_TYPE.calculate(expected_prob, isolated_prob)
        },
    )
}

enum ErrorType {
    SquaredRelative,
    SquaredAbsolute,
}
impl ErrorType {
    fn calculate(&self, expected: f64, sample: f64) -> f64 {
        match self {
            ErrorType::SquaredRelative => ((expected - sample) / sample).powi(2),
            ErrorType::SquaredAbsolute => (expected - sample).powi(2),
        }
    }

    fn reverse(&self, error: f64) -> f64 {
        error.sqrt()
    }
}

fn expectation_from_lambdas(lambdas: &[f64]) -> f64 {
    assert_eq!(3, lambdas.len());
    lambdas[0] + lambdas[1] + 2.0 * lambdas[2]
}

/// Intervals.
fn interval_scoregrid(
    explore_intervals: Range<u8>,
    max_total_goals: u16,
    h1_probs: ScoringProbs,
    h2_probs: ScoringProbs,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_interval(
        INTERVALS as u8,
        explore_intervals,
        max_total_goals,
        h1_probs,
        h2_probs,
        scoregrid,
    );
    // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
}

/// Binomial.
fn binomial_scoregrid(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_binomial(intervals, interval_home_prob, interval_away_prob, scoregrid);
    // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
}

/// Bivariate binomial.
fn bivariate_binomial_scoregrid(
    intervals: u8,
    interval_home_prob: f64,
    interval_away_prob: f64,
    interval_common_prob: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_bivariate_binomial(
        intervals,
        interval_home_prob,
        interval_away_prob,
        interval_common_prob,
        scoregrid,
    );
    // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
}

/// Independent Poisson.
fn univariate_poisson_scoregrid(home_rate: f64, away_rate: f64, scoregrid: &mut Matrix<f64>) {
    scoregrid.fill(0.0);
    scoregrid::from_univariate_poisson(home_rate, away_rate, scoregrid);
}

/// Bivariate Poisson.
fn bivariate_poisson_scoregrid(
    home_rate: f64,
    away_rate: f64,
    common: f64,
    scoregrid: &mut Matrix<f64>,
) {
    scoregrid.fill(0.0);
    scoregrid::from_bivariate_poisson(home_rate, away_rate, common, scoregrid);
    // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
}

fn correct_score_scoregrid(correct_score: &LabelledMarket, scoregrid: &mut Matrix<f64>) {
    scoregrid.fill(0.0);
    from_correct_score(
        &correct_score.outcomes,
        &correct_score.market.probs,
        scoregrid,
    );
}

fn allocate_scoregrid(max_total_goals: u16) -> Matrix<f64> {
    let dim = usize::min(max_total_goals as usize, INTERVALS) + 1;
    Matrix::allocate(dim, dim)
}

fn frame_prices(
    scoregrid: &Matrix<f64>,
    outcomes: &[OutcomeType],
    overround: &Overround,
) -> Market {
    let mut probs = outcomes
        .iter()
        .map(|outcome| outcome.gather(scoregrid))
        .map(|prob| f64::max(0.0001, prob))
        .collect::<Vec<_>>();
    probs.normalise(1.0);
    Market::frame(overround, probs, &SINGLE_PRICE_BOUNDS)
}

struct MarketErrors {
    rmse: f64,
    rmsre: f64,
}

fn compute_error(sample_prices: &[f64], fitted_prices: &[f64], error_type: &ErrorType) -> f64 {
    let mut error_sum = 0.0;
    let mut counted = 0;
    for (index, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[index];
        if fitted_price.is_finite() {
            counted += 1;
            let (sample_prob, fitted_prob) = (1.0 / sample_price, 1.0 / fitted_price);
            error_sum += error_type.calculate(sample_prob, fitted_prob);
        }
    }
    let mean_error = error_sum / counted as f64;
    error_type.reverse(mean_error)
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

fn print_errors(errors: &[(&MarketType, MarketErrors)]) -> Table {
    let mut table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(Left)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Market".into(), "RMSRE".into(), "RMSE".into()],
        ));
    for (market_type, error) in errors {
        table.push_row(Row::new(
            Styles::default(),
            vec![
                format!("{:?}", market_type).into(),
                format!("{:.3}", error.rmsre).into(),
                format!("{:.3}", error.rmse).into(),
            ],
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
            vec![
                format!("{:?}", market.market_type).into(),
                format!("{:.3}", market.market.overround.value).into(),
            ],
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
