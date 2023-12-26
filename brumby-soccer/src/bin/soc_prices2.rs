use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use tracing::{debug, info};

use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby::probs::SliceExt;
use brumby::sv;
use brumby_soccer::data::{download_by_id, ContestSummary, SoccerFeedId};
use brumby_soccer::domain::{Offer, OfferType, OutcomeType, Over, Period, Score};
use brumby_soccer::fit::{away_booksum, home_booksum, ErrorType, FittingErrors};
use brumby_soccer::interval::query::isolate;
use brumby_soccer::interval::{
    explore, BivariateProbs, Expansions, Exploration, Config, PlayerProbs, PruneThresholds,
    TeamProbs, UnivariateProbs,
};
use brumby_soccer::{fit, print};

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.01..=301.0;
const FIRST_GOALSCORER_BOOKSUM: f64 = 1.5;
const INTERVALS: u8 = 18;
const MAX_TOTAL_GOALS_HALF: u16 = 6;
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
    for offer_type in contest.offerings.keys() {
        info!("offered {offer_type:?}");
    }

    let ft_h2h_prices = contest.offerings[&OfferType::HeadToHead(Period::FullTime)].clone();
    let ft_goals_prices =
        contest.offerings[&OfferType::TotalGoals(Period::FullTime, Over(2))].clone();
    let ft_correct_score_prices =
        contest.offerings[&OfferType::CorrectScore(Period::FullTime)].clone();
    let h1_h2h_prices = contest.offerings[&OfferType::HeadToHead(Period::FirstHalf)].clone();
    let h1_goals_prices =
        contest.offerings[&OfferType::TotalGoals(Period::FirstHalf, Over(2))].clone();
    let h2_h2h_prices = contest.offerings[&OfferType::HeadToHead(Period::SecondHalf)].clone();
    let h2_goals_prices =
        contest.offerings[&OfferType::TotalGoals(Period::SecondHalf, Over(2))].clone();
    let first_gs = contest.offerings[&OfferType::FirstGoalscorer].clone();
    let anytime_gs = contest.offerings[&OfferType::AnytimeGoalscorer].clone();
    let anytime_assist = contest.offerings[&OfferType::AnytimeAssist].clone();

    let ft_h2h = fit_offer(OfferType::HeadToHead(Period::FullTime), &ft_h2h_prices, 1.0);
    // println!("ft_h2h: {ft_h2h:?}");
    let ft_goals = fit_offer(
        OfferType::TotalGoals(Period::FullTime, Over(2)),
        &ft_goals_prices,
        1.0,
    );
    // println!("ft_goals_ou: {ft_goals_ou:?}");
    let ft_correct_score = fit_offer(
        OfferType::CorrectScore(Period::FullTime),
        &ft_correct_score_prices,
        1.0,
    );
    let h1_h2h = fit_offer(
        OfferType::HeadToHead(Period::FirstHalf),
        &h1_h2h_prices,
        1.0,
    );
    let h1_goals = fit_offer(
        OfferType::TotalGoals(Period::FirstHalf, Over(2)),
        &h1_goals_prices,
        1.0,
    );
    let h2_h2h = fit_offer(
        OfferType::HeadToHead(Period::SecondHalf),
        &h2_h2h_prices,
        1.0,
    );
    let h2_goals = fit_offer(
        OfferType::TotalGoals(Period::SecondHalf, Over(2)),
        &h2_goals_prices,
        1.0,
    );

    let (ft_search_outcome, lambdas) = fit::fit_scoregrid_full(&ft_h2h, &ft_goals, INTERVALS, MAX_TOTAL_GOALS_FULL);
    const H1_RATIO: f64 = 0.425;
    println!("*** fitting H1 ***");
    let h1_home_goals_estimate = (lambdas[0] + lambdas[2]) * H1_RATIO;
    let h1_away_goals_estimate = (lambdas[1] + lambdas[2]) * H1_RATIO;
    let h1_search_outcome = fit::fit_scoregrid_half(h1_home_goals_estimate, h1_away_goals_estimate, &[&h1_h2h, &h1_goals], INTERVALS, MAX_TOTAL_GOALS_HALF);
    println!("*** fitting H2 ***");
    let h2_home_goals_estimate = (lambdas[0] + lambdas[2]) * (1.0 - H1_RATIO);
    let h2_away_goals_estimate = (lambdas[1] + lambdas[2]) * (1.0 - H1_RATIO);
    let h2_search_outcome = fit::fit_scoregrid_half(h2_home_goals_estimate, h2_away_goals_estimate, &[&h2_h2h, &h2_goals], INTERVALS, MAX_TOTAL_GOALS_HALF);

    let mut adj_optimal_h1 = [0.0; 3];
    let mut adj_optimal_h2 = [0.0; 3];
    // only adjust the home and away scoring probs; common prob is locked to the full-time one
    for (i, orig_h1) in h1_search_outcome.optimal_values.iter().take(2).enumerate() {
        let orig_h2 = h2_search_outcome.optimal_values[i];
        let ft = ft_search_outcome.optimal_values[i];
        let avg_h1_h2 = (orig_h1 + orig_h2) / 2.0;
        if avg_h1_h2 > 0.0 {
            adj_optimal_h1[i] = orig_h1 / (avg_h1_h2 / ft);
            adj_optimal_h2[i] = orig_h2 / (avg_h1_h2 / ft);
        } else {
            adj_optimal_h1[i] = ft;
            adj_optimal_h2[i] = ft;
        }
    }
    adj_optimal_h1[2] = ft_search_outcome.optimal_values[2];
    adj_optimal_h2[2] = ft_search_outcome.optimal_values[2];
    println!("adjusted optimal_h1={adj_optimal_h1:?}, optimal_h2={adj_optimal_h2:?}");
    // let adj_optimal_h1 = h1_search_outcome.optimal_values;
    // let adj_optimal_h2 = h2_search_outcome.optimal_values;

    // let ft_gamma_sum = ft_search_outcome.optimal_values.sum();
    // h1_search_outcome.optimal_values.normalise(ft_gamma_sum * 1.0);
    // h2_search_outcome.optimal_values.normalise(ft_gamma_sum * 1.0);

    let exploration = explore_scores(
        BivariateProbs::from(adj_optimal_h1.as_slice()),
        BivariateProbs::from(adj_optimal_h2.as_slice()),
    );

    // let mut ft_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_FULL);
    // interval_scoregrid(
    //     0..INTERVALS as u8,
    //     ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //     ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //     &mut ft_scoregrid,
    // );
    // interval_scoregrid(
    //     0..INTERVALS as u8,
    //     MAX_TOTAL_GOALS_FULL,
    //     ScoringProbs::from(adj_optimal_h1.as_slice()),
    //     ScoringProbs::from(adj_optimal_h2.as_slice()),
    //     &mut ft_scoregrid,
    // );
    // correct_score_scoregrid(&ft_correct_score, &mut ft_scoregrid);

    // let mut h1_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_HALF);
    // interval_scoregrid(
    //     0..(INTERVALS / 2) as u8,
    //     MAX_TOTAL_GOALS_HALF,
    //     ScoringProbs::from(adj_optimal_h1.as_slice()),
    //     ScoringProbs {
    //         home_prob: 0.0,
    //         away_prob: 0.0,
    //         common_prob: 0.0,
    //     },
    //     &mut h1_scoregrid,
    // );

    // let fitted_h1_h2h =
    //     frame_prices_from_scoregrid(&h1_scoregrid, &h1_h2h.outcomes.items(), &h1_h2h.market.overround);
    // let fitted_h1_h2h = Offer {
    //     offer_type: OfferType::HeadToHead(Period::FirstHalf),
    //     outcomes: h1_h2h.outcomes.clone(),
    //     market: fitted_h1_h2h,
    // };
    let fitted_h1_h2h = frame_prices_from_exploration(
        &exploration,
        &OfferType::HeadToHead(Period::FirstHalf),
        h1_h2h.outcomes.items(),
        1.0,
        &h1_h2h.market.overround,
    );
    let h1_h2h_table = print::tabulate_offer(&fitted_h1_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h1_h2h.offer_type,
        fitted_h1_h2h.market.probs.sum(),
        Console::default().render(&h1_h2h_table)
    );

    // let fitted_h1_goals_ou = frame_prices_from_scoregrid(
    //     &h1_scoregrid,
    //     &ft_goals_ou.outcomes,
    //     &ft_goals_ou.market.overround,
    // );
    // let fitted_h1_goals_ou = Offer {
    //     offer_type: OfferType::TotalGoals(Period::FirstHalf, Over(2)),
    //     outcomes: h1_goals_ou.outcomes.clone(),
    //     market: fitted_h1_goals_ou,
    // };
    let fitted_h1_goals = frame_prices_from_exploration(
        &exploration,
        &OfferType::TotalGoals(Period::FirstHalf, Over(2)),
        h1_goals.outcomes.items(),
        1.0,
        &h1_goals.market.overround,
    );
    let h1_goals_table = print::tabulate_offer(&fitted_h1_goals);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h1_goals.offer_type,
        fitted_h1_goals.market.probs.sum(),
        Console::default().render(&h1_goals_table)
    );

    // let mut h2_scoregrid = allocate_scoregrid(MAX_TOTAL_GOALS_HALF);
    // interval_scoregrid(
    //     0..(INTERVALS / 2) as u8,
    //     ModelParams { home_prob: h2_search_outcome.optimal_values[0], away_prob: h2_search_outcome.optimal_values[1], common_prob: h2_search_outcome.optimal_values[2] },
    //     ModelParams { home_prob: 0.0, away_prob: 0.0, common_prob: 0.0 },
    //     &mut h2_scoregrid,
    // );
    // interval_scoregrid(
    //     (INTERVALS / 2) as u8..INTERVALS as u8,
    //     MAX_TOTAL_GOALS_HALF,
    //     ScoringProbs {
    //         home_prob: 0.0,
    //         away_prob: 0.0,
    //         common_prob: 0.0,
    //     },
    //     ScoringProbs::from(adj_optimal_h2.as_slice()),
    //     &mut h2_scoregrid,
    // );

    // let fitted_h2_h2h =
    //     frame_prices_from_scoregrid(&h2_scoregrid, &h2_h2h.outcomes, &h2_h2h.market.overround);
    // let fitted_h2_h2h = Offer {
    //     offer_type: OfferType::HeadToHead(Period::SecondHalf),
    //     outcomes: h2_h2h.outcomes.clone(),
    //     market: fitted_h2_h2h,
    // };
    let fitted_h2_h2h = frame_prices_from_exploration(
        &exploration,
        &OfferType::HeadToHead(Period::SecondHalf),
        h2_h2h.outcomes.items(),
        1.0,
        &h2_h2h.market.overround,
    );
    let h2_h2h_table = print::tabulate_offer(&fitted_h2_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h2_h2h.offer_type,
        fitted_h2_h2h.market.probs.sum(),
        Console::default().render(&h2_h2h_table)
    );

    // let fitted_h2_goals_ou = frame_prices_from_scoregrid(
    //     &h2_scoregrid,
    //     &h2_goals_ou.outcomes,
    //     &h2_goals_ou.market.overround,
    // );
    // let fitted_h2_goals_ou = Offer {
    //     offer_type: OfferType::TotalGoals(Period::SecondHalf, Over(2)),
    //     outcomes: h2_goals_ou.outcomes.clone(),
    //     market: fitted_h2_goals_ou,
    // };
    let fitted_h2_goals = frame_prices_from_exploration(
        &exploration,
        &OfferType::TotalGoals(Period::SecondHalf, Over(2)),
        h2_goals.outcomes.items(),
        1.0,
        &h2_goals.market.overround,
    );
    let h2_goals_table = print::tabulate_offer(&fitted_h2_goals);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_h2_goals.offer_type,
        fitted_h2_goals.market.probs.sum(),
        Console::default().render(&h2_goals_table)
    );

    // let fitted_ft_h2h =
    //     frame_prices_from_scoregrid(&ft_scoregrid, &ft_h2h.outcomes, &ft_h2h.market.overround);
    // let fitted_ft_h2h = Offer {
    //     offer_type: OfferType::HeadToHead(Period::FullTime),
    //     outcomes: ft_h2h.outcomes.clone(),
    //     market: fitted_ft_h2h,
    // };
    let fitted_ft_h2h = frame_prices_from_exploration(
        &exploration,
        &OfferType::HeadToHead(Period::FullTime),
        ft_h2h.outcomes.items(),
        1.0,
        &ft_h2h.market.overround,
    );
    let ft_h2h_table = print::tabulate_offer(&fitted_ft_h2h);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_h2h.offer_type,
        fitted_ft_h2h.market.probs.sum(),
        Console::default().render(&ft_h2h_table)
    );

    let fitted_ft_goals_ou = frame_prices_from_exploration(
        &exploration,
        &OfferType::TotalGoals(Period::FullTime, Over(2)),
        ft_goals.outcomes.items(),
        1.0,
        &ft_goals.market.overround,
    );
    let ft_goals_ou_table = print::tabulate_offer(&fitted_ft_goals_ou);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_goals_ou.offer_type,
        fitted_ft_goals_ou.market.probs.sum(),
        Console::default().render(&ft_goals_ou_table)
    );

    let fitted_ft_correct_score = frame_prices_from_exploration(
        &exploration,
        &OfferType::CorrectScore(Period::FullTime),
        ft_correct_score.outcomes.items(),
        1.0,
        &ft_correct_score.market.overround,
    );
    let ft_correct_score_table = print::tabulate_offer(&fitted_ft_correct_score);
    println!(
        "{:?}: [Σ={:.3}]\n{}",
        fitted_ft_correct_score.offer_type,
        fitted_ft_correct_score.market.probs.sum(),
        Console::default().render(&ft_correct_score_table),
    );

    // let home_away_expectations = home_away_expectations(&ft_scoregrid);
    // println!(
    //     "p(0, 0)={}, home + away expectations: ({} + {} = {})",
    //     ft_scoregrid[(0, 0)],
    //     home_away_expectations.0,
    //     home_away_expectations.1,
    //     home_away_expectations.0 + home_away_expectations.1
    // );

    let first_gs = fit_offer(
        OfferType::FirstGoalscorer,
        &first_gs,
        FIRST_GOALSCORER_BOOKSUM,
    );
    let anytime_gs = fit_offer(OfferType::AnytimeGoalscorer, &anytime_gs, 1.0);

    // println!("scoregrid:\n{}sum: {}", scoregrid.verbose(), scoregrid.flatten().sum());
    let nil_all_draw_prob = isolate(
        &OfferType::CorrectScore(Period::FullTime),
        &OutcomeType::Score(Score { home: 0, away: 0 }),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    // let home_ratio = (ft_search_outcome.optimal_values[0]
    //     + ft_search_outcome.optimal_values[2] / 2.0)
    //     / ft_search_outcome.optimal_values.sum()
    //     * (1.0 - draw_prob);
    // let away_ratio = (ft_search_outcome.optimal_values[1]
    //     + ft_search_outcome.optimal_values[2] / 2.0)
    //     / ft_search_outcome.optimal_values.sum()
    //     * (1.0 - draw_prob);
    // // println!("home_ratio={home_ratio} + away_ratio={away_ratio}");
    // let mut fitted_goalscorer_probs = BTreeMap::new();
    // let start = Instant::now();
    // for (index, outcome) in first_gs.outcomes.items().iter().enumerate() {
    //     match outcome {
    //         OutcomeType::Player(player) => {
    //             let side_ratio = match player {
    //                 Named(side, _) => match side {
    //                     Side::Home => home_ratio,
    //                     Side::Away => away_ratio,
    //                 },
    //                 Player::Other => unreachable!(),
    //             };
    //             let init_estimate = first_gs.market.probs[index] / side_ratio;
    //             let player_search_outcome = fit::fit_first_goalscorer_one(
    //                 // &ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //                 //  &ModelParams { home_prob: ft_search_outcome.optimal_values[0], away_prob: ft_search_outcome.optimal_values[1], common_prob: ft_search_outcome.optimal_values[2] },
    //                 &ScoringProbs::from(adj_optimal_h1.as_slice()),
    //                 &ScoringProbs::from(adj_optimal_h2.as_slice()),
    //                 player,
    //                 init_estimate,
    //                 first_gs.market.probs[index],
    //             );
    //             // println!("for player {player:?}, {player_search_outcome:?}, sample prob. {}, init_estimate: {init_estimate}", first_gs.market.probs[index]);
    //             fitted_goalscorer_probs.insert(player.clone(), player_search_outcome.optimal_value);
    //         }
    //         OutcomeType::None => {}
    //         _ => unreachable!(),
    //     }
    // }
    // let elapsed = start.elapsed();
    // println!("player fitting took {elapsed:?}");

    debug!("nil-all draw prob: {nil_all_draw_prob}");
    let fitted_goalscorer_probs = fit::fit_first_goalscorer_all(
        &BivariateProbs::from(adj_optimal_h1.as_slice()),
        &BivariateProbs::from(adj_optimal_h2.as_slice()),
        &first_gs,
        nil_all_draw_prob,
        INTERVALS,
        MAX_TOTAL_GOALS_FULL
    );

    let mut fitted_first_goalscorer_probs = Vec::with_capacity(first_gs.outcomes.len());
    for (player, prob) in &fitted_goalscorer_probs {
        let exploration = explore(
            &Config {
                intervals: INTERVALS,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs::from(adj_optimal_h1.as_slice()),
                    h2_goals: BivariateProbs::from(adj_optimal_h2.as_slice()),
                    assists: UnivariateProbs {
                        home: 1.0,
                        away: 1.0,
                    },
                },
                player_probs: sv![(
                    player.clone(),
                    PlayerProbs {
                        goal: Some(*prob),
                        assist: None,
                    },
                )],
                prune_thresholds: PruneThresholds {
                    max_total_goals: MAX_TOTAL_GOALS_FULL,
                    min_prob: GOALSCORER_MIN_PROB,
                },
                expansions: Expansions {
                    ht_score: false,
                    ft_score: false,
                    max_player_goals: 0,
                    player_split_goal_stats: false,
                    max_player_assists: 0,
                    first_goalscorer: true,
                },
            },
            0..INTERVALS,
        );
        let isolated_prob = isolate(
            &OfferType::FirstGoalscorer,
            &OutcomeType::Player(player.clone()),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        fitted_first_goalscorer_probs.push(isolated_prob);
        // println!("first scorer {player:?}, prob: {isolated_prob:.3}");
    }

    // fitted_first_goalscorer_probs.push(draw_prob);
    // fitted_first_goalscorer_probs.normalise(FIRST_GOALSCORER_BOOKSUM);
    fitted_first_goalscorer_probs
        .push(FIRST_GOALSCORER_BOOKSUM - fitted_first_goalscorer_probs.sum());

    let fitted_first_goalscorer = Offer {
        offer_type: OfferType::FirstGoalscorer,
        outcomes: first_gs.outcomes.clone(),
        market: Market::frame(
            &first_gs.market.overround,
            fitted_first_goalscorer_probs,
            &SINGLE_PRICE_BOUNDS,
        ),
    };

    if args.player_goals {
        println!(
            "sample first goalscorer σ={:.3}",
            first_gs.market.offered_booksum(),
        );
        let table_first_goalscorer = print::tabulate_offer(&fitted_first_goalscorer);
        println!(
            "{:?}: [Σ={:.3}, σ={:.3}, n={}]\n{}",
            fitted_first_goalscorer.offer_type,
            fitted_first_goalscorer.market.probs.sum(),
            fitted_first_goalscorer.market.offered_booksum(),
            fitted_first_goalscorer.market.probs.len(),
            Console::default().render(&table_first_goalscorer)
        );
    }

    let mut fitted_anytime_goalscorer_outcomes =
        HashLookup::with_capacity(fitted_goalscorer_probs.len());
    let mut fitted_anytime_goalscorer_probs = Vec::with_capacity(fitted_goalscorer_probs.len());
    for (player, prob) in &fitted_goalscorer_probs {
        fitted_anytime_goalscorer_outcomes.push(OutcomeType::Player(player.clone()));
        let exploration = explore(
            &Config {
                intervals: INTERVALS,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs::from(adj_optimal_h1.as_slice()),
                    h2_goals: BivariateProbs::from(adj_optimal_h2.as_slice()),
                    assists: UnivariateProbs {
                        home: 1.0,
                        away: 1.0,
                    },
                },
                player_probs: sv![(
                    player.clone(),
                    PlayerProbs {
                        goal: Some(*prob),
                        assist: None,
                    },
                )],
                prune_thresholds: PruneThresholds {
                    max_total_goals: MAX_TOTAL_GOALS_FULL,
                    min_prob: GOALSCORER_MIN_PROB,
                },
                expansions: Expansions {
                    ht_score: false,
                    ft_score: false,
                    max_player_goals: u8::MAX,
                    player_split_goal_stats: false,
                    max_player_assists: 0,
                    first_goalscorer: false,
                },
            },
            0..INTERVALS,
        );
        let isolated_prob = isolate(
            &OfferType::AnytimeGoalscorer,
            &OutcomeType::Player(player.clone()),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        fitted_anytime_goalscorer_probs.push(isolated_prob);
    }
    fitted_anytime_goalscorer_outcomes.push(OutcomeType::None);
    fitted_anytime_goalscorer_probs.push(nil_all_draw_prob);

    let anytime_goalscorer_overround = Overround {
        method: OVERROUND_METHOD,
        value: anytime_gs.market.offered_booksum() / fitted_anytime_goalscorer_probs.sum(),
    };
    let fitted_anytime_goalscorer = Offer {
        offer_type: OfferType::AnytimeGoalscorer,
        outcomes: fitted_anytime_goalscorer_outcomes,
        market: Market::frame(
            &anytime_goalscorer_overround,
            fitted_anytime_goalscorer_probs,
            &SINGLE_PRICE_BOUNDS,
        ),
    };

    if args.player_goals {
        println!(
            "sample anytime goalscorer σ={:.3}",
            anytime_gs.market.offered_booksum(),
        );
        let table_anytime_goalscorer = print::tabulate_offer(&fitted_anytime_goalscorer);
        println!(
            "{:?}: [Σ={:.3}, σ={:.3}, n={}]\n{}",
            fitted_anytime_goalscorer.offer_type,
            fitted_anytime_goalscorer.market.probs.sum(),
            fitted_anytime_goalscorer.market.offered_booksum(),
            fitted_anytime_goalscorer.market.probs.len(),
            Console::default().render(&table_anytime_goalscorer)
        );
    }

    let sample_anytime_assist_booksum = anytime_assist
        .values()
        .map(|price| 1.0 / price)
        .sum::<f64>();

    let per_outcome_overround =
        (anytime_goalscorer_overround.value - 1.0) / anytime_gs.outcomes.len() as f64;

    let anytime_assist = fit_offer(
        OfferType::AnytimeAssist,
        &anytime_assist,
        sample_anytime_assist_booksum / (1.0 + per_outcome_overround * anytime_assist.len() as f64),
    );

    let home_goalscorer_booksum = home_booksum(&fitted_anytime_goalscorer);
    let away_goalscorer_booksum = away_booksum(&fitted_anytime_goalscorer);
    // println!("partial goalscorer booksums: home: {home_goalscorer_booksum:.3}, away: {away_goalscorer_booksum:.3}");

    let home_assister_booksum = home_booksum(&anytime_assist);
    let away_assister_booksum = away_booksum(&anytime_assist);
    // println!("partial assister booksums: home: {home_assister_booksum:.3}, away: {away_assister_booksum:.3}");
    let assist_probs = UnivariateProbs {
        home: home_assister_booksum / home_goalscorer_booksum,
        away: away_assister_booksum / away_goalscorer_booksum,
    };
    println!("assist_probs: {assist_probs:?}");

    let fitted_assist_probs = fit::fit_anytime_assist_all(
        &BivariateProbs::from(adj_optimal_h1.as_slice()),
        &BivariateProbs::from(adj_optimal_h2.as_slice()),
        &assist_probs,
        &anytime_assist,
        nil_all_draw_prob,
        anytime_assist.market.fair_booksum(),
        INTERVALS,
        MAX_TOTAL_GOALS_FULL
    );

    let mut fitted_anytime_assist_outcomes = HashLookup::with_capacity(fitted_assist_probs.len());
    let mut fitted_anytime_assist_probs = Vec::with_capacity(fitted_assist_probs.len());
    for (player, prob) in &fitted_assist_probs {
        fitted_anytime_assist_outcomes.push(OutcomeType::Player(player.clone()));
        let exploration = explore(
            &Config {
                intervals: INTERVALS,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs::from(adj_optimal_h1.as_slice()),
                    h2_goals: BivariateProbs::from(adj_optimal_h2.as_slice()),
                    assists: assist_probs.clone(),
                },
                player_probs: sv![(
                    player.clone(),
                    PlayerProbs {
                        goal: None,
                        assist: Some(*prob),
                    },
                )],
                prune_thresholds: PruneThresholds {
                    max_total_goals: MAX_TOTAL_GOALS_FULL,
                    min_prob: GOALSCORER_MIN_PROB,
                },
                expansions: Expansions {
                    ht_score: false,
                    ft_score: false,
                    max_player_goals: 0,
                    player_split_goal_stats: false,
                    max_player_assists: 1,
                    first_goalscorer: false,
                },
            },
            0..INTERVALS,
        );
        let isolated_prob = isolate(
            &OfferType::AnytimeAssist,
            &OutcomeType::Player(player.clone()),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        fitted_anytime_assist_probs.push(isolated_prob);
    }

    let anytime_assist_overround = Overround {
        method: OVERROUND_METHOD,
        value: anytime_assist.market.offered_booksum() / fitted_anytime_assist_probs.sum(),
    };
    fitted_anytime_assist_outcomes.push(OutcomeType::None);
    fitted_anytime_assist_probs.push(nil_all_draw_prob);

    let fitted_anytime_assist = Offer {
        offer_type: OfferType::AnytimeAssist,
        outcomes: fitted_anytime_assist_outcomes,
        market: Market::frame(
            &anytime_assist_overround,
            fitted_anytime_assist_probs,
            &SINGLE_PRICE_BOUNDS,
        ),
    };

    if args.player_assists {
        println!(
            "sample anytime assists σ={:.3}",
            anytime_assist.market.offered_booksum(),
        );
        let table_anytime_assist = print::tabulate_offer(&fitted_anytime_assist);
        println!(
            "{:?}: [Σ={:.3}, σ={:.3}, n={}]\n{}",
            fitted_anytime_assist.offer_type,
            fitted_anytime_assist.market.probs.sum(),
            fitted_anytime_assist.market.offered_booksum(),
            fitted_anytime_assist.market.probs.len(),
            Console::default().render(&table_anytime_assist)
        );
    }

    let market_errors = [
        (&h1_h2h, &fitted_h1_h2h),
        (&h1_goals, &fitted_h1_goals),
        (&h2_h2h, &fitted_h2_h2h),
        (&h2_goals, &fitted_h2_goals),
        (&ft_h2h, &fitted_ft_h2h),
        (&ft_goals, &fitted_ft_goals_ou),
        (&ft_correct_score, &fitted_ft_correct_score),
        (&first_gs, &fitted_first_goalscorer),
        (&anytime_gs, &fitted_anytime_goalscorer),
        (&anytime_assist, &fitted_anytime_assist),
    ]
    .iter()
    .map(|(sample, fitted)| {
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
    let table_errors = print::tabulate_errors(&market_errors);
    println!(
        "Fitting errors:\n{}",
        Console::default().render(&table_errors)
    );

    let fitted_markets = [
        &fitted_h1_h2h,
        &fitted_h1_goals,
        &fitted_h2_h2h,
        &fitted_h2_goals,
        &fitted_ft_h2h,
        &fitted_ft_goals_ou,
        &fitted_ft_correct_score,
        &fitted_first_goalscorer,
        &fitted_anytime_goalscorer,
        &fitted_anytime_assist,
    ];

    let table_overrounds = print::tabulate_overrounds(&fitted_markets);
    println!(
        "Market overrounds:\n{}",
        Console::default().render(&table_overrounds)
    );

    // let fitted_markets_hash = fitted_markets.iter().map(|market| (market.offer_type.clone(), market)).collect::<HashMap<_, _>>();
    // let h2h_sel = (OfferType::HeadToHead(Period::FullTime), OutcomeType::Win(Side::Home));
    // let tg_sel = (OfferType::TotalGoalsOverUnder(Period::FullTime, Over(2)), OutcomeType::Over(2));
    // let anytime_player = Named(Side::Home, "Gianluca Lapadula".into());
    // let anytime_sel = (OfferType::AnytimeGoalscorer, OutcomeType::Player(anytime_player.clone()));
    // let player_prob = fitted_goalscorer_probs[&anytime_player];
    // let exploration = explore(
    //     &IntervalConfig {
    //         intervals: INTERVALS as u8,
    //         h1_probs: ScoringProbs::from(adj_optimal_h1.as_slice()),
    //         h2_probs: ScoringProbs::from(adj_optimal_h2.as_slice()),
    //         players: vec![
    //             (anytime_player.clone(), player_prob)
    //         ],
    //         prune_thresholds: PruneThresholds {
    //             max_total_goals: MAX_TOTAL_GOALS_FULL,
    //             min_prob: 0.0,
    //         },
    //         expansions: Expansions::default()
    //     },
    //     0..INTERVALS as u8,
    // );
    // let selections = [h2h_sel, tg_sel, anytime_sel];
    // let overround = selections.iter().map(|(offer_type, outcome_type)| {
    //     let fitted_market = fitted_markets_hash[offer_type];
    //     let outcome_index = fitted_market.outcomes.iter().position(|in_vec| in_vec == outcome_type).unwrap();
    //     let outcome_prob = fitted_market.market.probs[outcome_index];
    //     let outcome_price = fitted_market.market.prices[outcome_index];
    //     1.0 / outcome_prob / outcome_price
    // }).product::<f64>();
    // let multi_prob = isolate_batch(&selections, &exploration.prospects, &exploration.player_lookup);
    // let multi_price = 1.0 / multi_prob / overround;
    // info!("selections: {selections:?}, prob: {multi_prob:.3}, overround: {overround:.3}, price: {multi_price:.3}");

    Ok(())
}

// fn implied_booksum(prices: &[f64]) -> f64 {
//     prices.invert().sum()
// }

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

// fn fit_scoregrid_half(markets: &[&Offer]) -> HypergridSearchOutcome {
//     let init_estimates = {
//         let start = Instant::now();
//         let search_outcome = fit_bivariate_poisson_scoregrid(markets, MAX_TOTAL_GOALS_HALF);
//         let elapsed = start.elapsed();
//         println!("biv-poisson: {elapsed:?} elapsed: search outcome: {search_outcome:?}, expectation: {:.3}", expectation_from_lambdas(&search_outcome.optimal_values));
//         search_outcome
//             .optimal_values
//             .iter()
//             .map(|optimal_value| {
//                 1.0 - poisson::univariate(
//                     0,
//                     optimal_value / INTERVALS as f64 * 2.0,
//                     &factorial::Calculator,
//                 )
//             })
//             .collect::<Vec<_>>()
//     };
//     println!("initial estimates: {init_estimates:?}");
//
//     let start = Instant::now();
//     let search_outcome = fit_bivariate_binomial_scoregrid(
//         markets,
//         &init_estimates,
//         (INTERVALS / 2) as u8,
//         MAX_TOTAL_GOALS_HALF,
//     );
//     let elapsed = start.elapsed();
//     println!("biv-binomial: {elapsed:?} elapsed: search outcome: {search_outcome:?}");
//     search_outcome
// }

// fn fit_bivariate_binomial_scoregrid(
//     markets: &[&Offer],
//     init_estimates: &[f64],
//     intervals: u8,
//     max_total_goals: u16,
// ) -> HypergridSearchOutcome {
//     let mut scoregrid = allocate_scoregrid(max_total_goals);
//     let bounds = init_estimates
//         .iter()
//         .map(|&estimate| (estimate * 0.67)..=(estimate * 1.5))
//         .collect::<Vec<_>>();
//     hypergrid_search(
//         &HypergridSearchConfig {
//             max_steps: 10,
//             acceptable_residual: 1e-6,
//             bounds: bounds.into(),
//             resolution: 10,
//         },
//         |values| values.sum() <= 1.0,
//         |values| {
//             bivariate_binomial_scoregrid(
//                 intervals,
//                 values[0],
//                 values[1],
//                 values[2],
//                 &mut scoregrid,
//             );
//             scoregrid_error(markets, &scoregrid)
//         },
//     )
// }
//
// fn fit_bivariate_poisson_scoregrid(
//     markets: &[&Offer],
//     max_total_goals: u16,
// ) -> HypergridSearchOutcome {
//     let mut scoregrid = allocate_scoregrid(max_total_goals);
//     hypergrid_search(
//         &HypergridSearchConfig {
//             max_steps: 10,
//             acceptable_residual: 1e-6,
//             bounds: vec![0.2..=3.0, 0.2..=3.0, 0.0..=0.5].into(),
//             resolution: 10,
//         },
//         |_| true,
//         |values| {
//             bivariate_poisson_scoregrid(values[0], values[1], values[2], &mut scoregrid);
//             scoregrid_error(markets, &scoregrid)
//         },
//     )
// }
//
// fn scoregrid_error(offers: &[&Offer], scoregrid: &Matrix<f64>) -> f64 {
//     let mut residual = 0.0;
//     for offer in offers {
//         for (index, outcome) in offer.outcomes.items().iter().enumerate() {
//             let fitted_prob = outcome.gather(scoregrid);
//             let sample_prob = offer.market.probs[index];
//             residual += ERROR_TYPE.calculate(sample_prob, fitted_prob);
//         }
//     }
//     residual
// }

// fn fit_first_goalscorer(
//     h1_probs: &ScoringProbs,
//     h2_probs: &ScoringProbs,
//     player: &Player,
//     init_estimate: f64,
//     expected_prob: f64,
// ) -> UnivariateDescentOutcome {
//     univariate_descent(
//         &UnivariateDescentConfig {
//             init_value: init_estimate,
//             init_step: init_estimate * 0.1,
//             min_step: init_estimate * 0.001,
//             max_steps: 100,
//             acceptable_residual: 1e-9,
//         },
//         |value| {
//             let exploration = explore(
//                 &IntervalConfig {
//                     intervals: INTERVALS as u8,
//                     h1_probs: h1_probs.clone(),
//                     h2_probs: h2_probs.clone(),
//                     players: vec![(player.clone(), value)],
//                     prune_thresholds: PruneThresholds {
//                         max_total_goals: MAX_TOTAL_GOALS_FULL,
//                         min_prob: GOALSCORER_MIN_PROB,
//                     },
//                     expansions: Expansions {
//                         ht_score: false,
//                         ft_score: false,
//                         player_stats: false,
//                         player_split_stats: false,
//                         first_goalscorer: true,
//                     },
//                 },
//                 0..INTERVALS as u8,
//             );
//             let isolated_prob = isolate(
//                 &OfferType::FirstGoalscorer,
//                 &OutcomeType::Player(player.clone()),
//                 &exploration.prospects,
//                 &exploration.player_lookup,
//             );
//             ERROR_TYPE.calculate(expected_prob, isolated_prob)
//         },
//     )
// }

// fn expectation_from_lambdas(lambdas: &[f64]) -> f64 {
//     assert_eq!(3, lambdas.len());
//     lambdas[0] + lambdas[1] + 2.0 * lambdas[2]
// }

// /// Intervals.
// fn interval_scoregrid(
//     explore_intervals: Range<u8>,
//     max_total_goals: u16,
//     h1_probs: ScoringProbs,
//     h2_probs: ScoringProbs,
//     scoregrid: &mut Matrix<f64>,
// ) {
//     scoregrid.fill(0.0);
//     scoregrid::from_interval(
//         INTERVALS as u8,
//         explore_intervals,
//         max_total_goals,
//         h1_probs,
//         h2_probs,
//         scoregrid,
//     );
//     // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
// }

fn explore_scores(h1_goals: BivariateProbs, h2_goals: BivariateProbs) -> Exploration {
    explore(
        &Config {
            intervals: INTERVALS,
            team_probs: TeamProbs {
                h1_goals,
                h2_goals,
                assists: UnivariateProbs {
                    home: 1.0,
                    away: 1.0,
                },
            },
            player_probs: sv![],
            prune_thresholds: PruneThresholds {
                max_total_goals: MAX_TOTAL_GOALS_FULL,
                min_prob: 0.0,
            },
            expansions: Expansions {
                ht_score: true,
                ft_score: true,
                max_player_goals: 0,
                player_split_goal_stats: false,
                max_player_assists: 0,
                first_goalscorer: false,
            },
        },
        0..INTERVALS,
    )
}

// /// Binomial.
// fn binomial_scoregrid(
//     intervals: u8,
//     interval_home_prob: f64,
//     interval_away_prob: f64,
//     scoregrid: &mut Matrix<f64>,
// ) {
//     scoregrid.fill(0.0);
//     scoregrid::from_binomial(intervals, interval_home_prob, interval_away_prob, scoregrid);
//     // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
// }
//
// /// Bivariate binomial.
// fn bivariate_binomial_scoregrid(
//     intervals: u8,
//     interval_home_prob: f64,
//     interval_away_prob: f64,
//     interval_common_prob: f64,
//     scoregrid: &mut Matrix<f64>,
// ) {
//     scoregrid.fill(0.0);
//     scoregrid::from_bivariate_binomial(
//         intervals,
//         interval_home_prob,
//         interval_away_prob,
//         interval_common_prob,
//         scoregrid,
//     );
//     // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
// }
//
// /// Independent Poisson.
// fn univariate_poisson_scoregrid(home_rate: f64, away_rate: f64, scoregrid: &mut Matrix<f64>) {
//     scoregrid.fill(0.0);
//     scoregrid::from_univariate_poisson(home_rate, away_rate, scoregrid);
// }
//
// /// Bivariate Poisson.
// fn bivariate_poisson_scoregrid(
//     home_rate: f64,
//     away_rate: f64,
//     common: f64,
//     scoregrid: &mut Matrix<f64>,
// ) {
//     scoregrid.fill(0.0);
//     scoregrid::from_bivariate_poisson(home_rate, away_rate, common, scoregrid);
//     // scoregrid::inflate_zero(ZERO_INFLATION, &mut scoregrid);
// }
//
// fn correct_score_scoregrid(correct_score: &Offer, scoregrid: &mut Matrix<f64>) {
//     scoregrid.fill(0.0);
//     from_correct_score(
//         correct_score.outcomes.items(),
//         &correct_score.market.probs,
//         scoregrid,
//     );
// }
//
// fn allocate_scoregrid(max_total_goals: u16) -> Matrix<f64> {
//     let dim = usize::min(max_total_goals as usize, INTERVALS) + 1;
//     Matrix::allocate(dim, dim)
// }
//
// fn frame_prices_from_scoregrid(
//     scoregrid: &Matrix<f64>,
//     outcomes: &[OutcomeType],
//     overround: &Overround,
// ) -> Market {
//     let mut probs = outcomes
//         .iter()
//         .map(|outcome_type| outcome_type.gather(scoregrid))
//         .map(|prob| f64::max(0.0001, prob))
//         .collect::<Vec<_>>();
//     probs.normalise(1.0);
//     Market::frame(overround, probs, &SINGLE_PRICE_BOUNDS)
// }

fn frame_prices_from_exploration(
    exploration: &Exploration,
    offer_type: &OfferType,
    outcomes: &[OutcomeType],
    normal: f64,
    overround: &Overround,
) -> Offer {
    let mut probs = outcomes
        .iter()
        .map(|outcome_type| {
            isolate(
                offer_type,
                outcome_type,
                &exploration.prospects,
                &exploration.player_lookup,
            )
        })
        // .map(|prob| f64::max(1e-6, prob))
        .collect::<Vec<_>>();
    probs.normalise(normal);
    let market = Market::frame(overround, probs, &SINGLE_PRICE_BOUNDS);
    Offer {
        offer_type: offer_type.clone(),
        outcomes: HashLookup::from(outcomes.to_vec()),
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
