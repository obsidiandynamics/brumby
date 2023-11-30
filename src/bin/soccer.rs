use std::collections::HashMap;
use std::time::Instant;

use HAlign::Left;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, MinWidth, Styles};
use stanza::table::{Col, Row, Table};

use brumby::linear::matrix::Matrix;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby::opt::{hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome};
use brumby::probs::SliceExt;
use brumby::scoregrid;
use brumby::scoregrid::{MarketType, OutcomeType, Over, Player, Score, Side};
use Player::Named;

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.04..=200.0;
const ZERO_INFLATION: f64 = 0.015;
const INTERVALS: usize = 10;
const MAX_TOTAL_GOALS: u16 = 8;

type Odds = HashMap<OutcomeType, f64>;

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

    HashMap::from([
        (MarketType::HeadToHead, h2h),
        (MarketType::TotalGoalsOverUnder(Over(2)), goals_ou),
        (MarketType::CorrectScore, correct_score),
        (MarketType::FirstGoalscorer, first_goalscorer),
    ])
}

pub fn main() {
    let ext_markets = atlanta_vs_sporting_lisbon();
    let correct_score_prices = ext_markets[&MarketType::CorrectScore].clone();
    let h2h_prices = ext_markets[&MarketType::HeadToHead].clone();
    let goals_ou_prices = ext_markets[&MarketType::TotalGoalsOverUnder(Over(2))].clone();
    let first_gs = ext_markets[&MarketType::FirstGoalscorer].clone();

    let h2h = fit_market(&h2h_prices);
    println!("h2h: {h2h:?}");
    let goals_ou = fit_market(&goals_ou_prices);
    println!("goals_ou: {goals_ou:?}");
    let correct_score = fit_market(&correct_score_prices);
    println!("correct_score: {correct_score:?}");
    let first_gs = fit_market(&first_gs);
    println!("first_gs: {first_gs:?}");

    println!("*** fitting scoregrid ***");
    let start = Instant::now();
    let search_outcome = fit_scoregrid(&[&h2h, &goals_ou]);
    let elapsed = start.elapsed();
    println!("{elapsed:?} elapsed: search outcome: {search_outcome:?}");

    let scoregrid = interval_scoregrid(
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
        search_outcome.optimal_values[2]
    );
    // println!("scoregrid:\n{}sum: {}", scoregrid.verbose(), scoregrid.flatten().sum());

    let fitted_h2h = frame_prices(&scoregrid, &h2h.outcomes, &h2h.market.overround);
    let fitted_h2h = LabelledMarket {
        outcomes: h2h.outcomes.clone(),
        market: fitted_h2h,
    };
    let table_h2h = print_market(&fitted_h2h);
    println!("H2H:\n{}", Console::default().render(&table_h2h));

    let fitted_goals_ou = frame_prices(&scoregrid, &goals_ou.outcomes, &goals_ou.market.overround);
    let fitted_goals_ou = LabelledMarket {
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
    ]
    .iter()
    .map(|(key, sample, fitted)| {
        (
            *key,
            compute_msre(&sample.market.prices, &fitted.market.prices).sqrt(),
        )
    })
    .collect::<Vec<_>>();
    let table_errors = print_errors(&market_errors);
    println!(
        "Fitting errors:\n{}",
        Console::default().render(&table_errors)
    );
}

fn fit_market(map: &HashMap<OutcomeType, f64>) -> LabelledMarket {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let outcomes = entries
        .iter()
        .map(|(outcome, _)| (*outcome).clone())
        .collect::<Vec<_>>();
    let prices = entries.iter().map(|(_, &price)| price).collect::<Vec<_>>();
    let market = Market::fit(&OVERROUND_METHOD, prices, 1.0);
    LabelledMarket { outcomes, market }
}

#[derive(Debug)]
pub struct LabelledMarket {
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
                    let relative_error = (sample_prob - fitted_prob) / sample_prob;
                    residual += relative_error.powi(2);
                    // residual += (sample_prob - fitted_prob).powi(2);
                }
            }
            residual
        },
    )
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

fn compute_msre(sample_prices: &[f64], fitted_prices: &[f64]) -> f64 {
    let mut sq_rel_error = 0.0;
    let mut counted = 0;
    for (index, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[index];
        if fitted_price.is_finite() {
            counted += 1;
            let relative_error = (sample_price - fitted_price) / sample_price;
            sq_rel_error += relative_error.powi(2);
        }
    }
    sq_rel_error / counted as f64
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
