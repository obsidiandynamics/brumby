use brumby::capture::Capture;
use brumby::linear::matrix::Matrix;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby::opt::{hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome};
use brumby::scoregrid;
use brumby::scoregrid::{Iter, IterFixtures, Outcome, Score, ScoreOutcomeSpace, Side};
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use std::collections::HashMap;
use HAlign::Left;
use brumby::probs::SliceExt;

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::OddsRatio;
const SINGLE_PRICE_BOUNDS: PriceBounds = 1.04..=200.0;

pub fn main() {
    let correct_score_prices = HashMap::from([
        // home wins
        (Outcome::CorrectScore(Score::new(1, 0)), 7.0),
        (Outcome::CorrectScore(Score::new(2, 0)), 12.0),
        (Outcome::CorrectScore(Score::new(2, 1)), 10.0),
        (Outcome::CorrectScore(Score::new(3, 0)), 26.0),
        (Outcome::CorrectScore(Score::new(3, 1)), 23.0),
        (Outcome::CorrectScore(Score::new(3, 2)), 46.0),
        (Outcome::CorrectScore(Score::new(4, 0)), 101.0),
        (Outcome::CorrectScore(Score::new(4, 1)), 81.0),
        (Outcome::CorrectScore(Score::new(4, 2)), 126.0),
        (Outcome::CorrectScore(Score::new(4, 3)), 200.0),
        (Outcome::CorrectScore(Score::new(5, 0)), 200.0),
        (Outcome::CorrectScore(Score::new(5, 1)), 200.0),
        (Outcome::CorrectScore(Score::new(5, 2)), 200.0),
        (Outcome::CorrectScore(Score::new(5, 3)), 200.0),
        (Outcome::CorrectScore(Score::new(5, 4)), 200.0),
        // draws
        (Outcome::CorrectScore(Score::new(0, 0)), 6.75),
        (Outcome::CorrectScore(Score::new(1, 1)), 5.90),
        (Outcome::CorrectScore(Score::new(2, 2)), 16.00),
        (Outcome::CorrectScore(Score::new(3, 3)), 101.00),
        (Outcome::CorrectScore(Score::new(4, 4)), 200.00),
        (Outcome::CorrectScore(Score::new(5, 5)), 200.00),
        // away wins
        (Outcome::CorrectScore(Score::new(0, 1)), 7.25),
        (Outcome::CorrectScore(Score::new(0, 2)), 12.0),
        (Outcome::CorrectScore(Score::new(1, 2)), 10.5),
        (Outcome::CorrectScore(Score::new(0, 3)), 31.0),
        (Outcome::CorrectScore(Score::new(1, 3)), 23.0),
        (Outcome::CorrectScore(Score::new(2, 3)), 41.0),
        (Outcome::CorrectScore(Score::new(0, 4)), 101.0),
        (Outcome::CorrectScore(Score::new(1, 4)), 81.0),
        (Outcome::CorrectScore(Score::new(2, 4)), 151.0),
        (Outcome::CorrectScore(Score::new(3, 4)), 200.0),
        (Outcome::CorrectScore(Score::new(0, 5)), 200.0),
        (Outcome::CorrectScore(Score::new(1, 5)), 200.0),
        (Outcome::CorrectScore(Score::new(2, 5)), 200.0),
        (Outcome::CorrectScore(Score::new(3, 5)), 200.0),
        (Outcome::CorrectScore(Score::new(4, 5)), 200.0),
    ]);

    let h2h_prices = HashMap::from([
        (Outcome::Win(Side::Home), 2.7),
        (Outcome::Draw, 2.87),
        (Outcome::Win(Side::Away), 2.87),
    ]);

    let goals_ou_prices =
        HashMap::from([(Outcome::GoalsOver(2), 2.47), (Outcome::GoalsUnder(3), 1.5)]);

    let h2h = fit_market(&h2h_prices);
    println!("h2h: {h2h:?}");
    let goals_ou = fit_market(&goals_ou_prices);
    println!("goals_ou: {goals_ou:?}");
    let correct_score = fit_market(&correct_score_prices);
    println!("correct_score: {correct_score:?}");

    let search_outcome = fit_scoregrid(&h2h, &goals_ou);
    println!("---");
    println!("search outcome: {search_outcome:?}");

    let scoregrid = interval_scoregrid(
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
        // search_outcome.optimal_values[2]
    );
    println!("scoregrid:\n{}sum: {}", scoregrid.verbose(), scoregrid.flatten().sum());

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

fn fit_market(map: &HashMap<Outcome, f64>) -> LabelledMarket {
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
    outcomes: Vec<Outcome>,
    market: Market,
}

fn fit_scoregrid(h2h: &LabelledMarket, goals_ou: &LabelledMarket) -> HypergridSearchOutcome {
    hypergrid_search(
        &HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-6,
            bounds: Capture::Owned(vec![0.0..=0.5, 0.0..=0.5]),
            // bounds: Capture::Owned(vec![0.5..=3.5, 0.5..=3.5]),
            // bounds: Capture::Owned(vec![0.5..=3.5, 0.5..=3.5, 0.0..=0.1]), // for the bivariate
            resolution: 4,
        },
        |values| {
            let scoregrid = binomial_scoregrid(values[0], values[1]);

            let mut residual = 0.0;
            for market in [&h2h, &goals_ou] {
                for (index, outcome) in market.outcomes.iter().enumerate() {
                    let fitted_prob = outcome.gather(&scoregrid);
                    let sample_prob = market.market.probs[index];
                    let relative_error = (sample_prob - fitted_prob) / sample_prob;
                    residual += relative_error.powi(2);
                }
            }
            residual
        },
    )
}

/// Intervals.
fn interval_scoregrid(interval_home_prob: f64, interval_away_prob: f64) -> Matrix<f64> {
    const INTERVALS: usize = 10;
    let space = ScoreOutcomeSpace {
        interval_home_prob,
        interval_away_prob,
    };
    let mut fixtures = IterFixtures::new(INTERVALS);
    let iter = Iter::new(&space, &mut fixtures);
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_iterator(iter, &mut scoregrid);
    scoregrid::inflate_zero(0.035, &mut scoregrid);
    scoregrid
}

/// Binomial.
fn binomial_scoregrid(interval_home_prob: f64, interval_away_prob: f64) -> Matrix<f64> {
    const INTERVALS: usize = 10;
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_binomial(interval_home_prob, interval_away_prob, &mut scoregrid);
    scoregrid::inflate_zero(0.035, &mut scoregrid);
    scoregrid
}

/// Independent Poisson.
fn univariate_poisson_scoregrid(home_rate: f64, away_rate: f64) -> Matrix<f64> {
    const INTERVALS: usize = 6;
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_univariate_poisson(home_rate, away_rate, &mut scoregrid);
    scoregrid::inflate_zero(0.035, &mut scoregrid);
    scoregrid
}

/// Bivariate Poisson.
fn bivariate_poisson_scoregrid(home_rate: f64, away_rate: f64, common: f64) -> Matrix<f64> {
    const INTERVALS: usize = 6;
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_bivariate_poisson(home_rate, away_rate, common, &mut scoregrid);
    scoregrid::inflate_zero(0.035, &mut scoregrid);
    scoregrid
}

fn frame_prices(scoregrid: &Matrix<f64>, outcomes: &[Outcome], overround: &Overround) -> Market {
    let probs = outcomes
        .iter()
        .map(|outcome| outcome.gather(scoregrid))
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
