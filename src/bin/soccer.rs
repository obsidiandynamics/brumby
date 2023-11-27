use brumby::capture::Capture;
use brumby::linear::matrix::Matrix;
use brumby::market::{Market, Overround, OverroundMethod};
use brumby::opt::{hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome};
use brumby::scoregrid;
use brumby::scoregrid::{Iter, IterFixtures, Outcome, Score, ScoreOutcomeSpace, Side};
use stanza::style::{HAlign, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use std::collections::HashMap;
use HAlign::Left;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

const OVERROUND_METHOD: OverroundMethod = OverroundMethod::Power;

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

    // let h2h_outcomes = h2h_market.keys().cloned().collect::<Vec<_>>();
    // let h2h_prices = h2h_market.values().copied().collect::<Vec<_>>();
    // let h2h_probs = Market::fit(&OverroundMethod::OddsRatio, h2h_prices, 1.0);
    //
    // let goals_ou_outcomes = goals_ou_market.keys().cloned().collect::<Vec<_>>();
    // let goals_ou_prices = goals_ou_market.values().copied().collect::<Vec<_>>();
    // let goals_ou_probs = Market::fit(&OverroundMethod::OddsRatio, goals_ou_prices, 1.0);

    let h2h = fit_market(&h2h_prices);
    println!("h2h: {h2h:?}");
    let goals_ou = fit_market(&goals_ou_prices);
    println!("goals_ou: {goals_ou:?}");
    let correct_score = fit_market(&correct_score_prices);
    println!("correct_score: {correct_score:?}");

    let search_outcome = fit_scoregrid(&h2h, &goals_ou);
    println!("search outcome: {search_outcome:?}");

    let scoregrid = create_scoregrid(
        search_outcome.optimal_values[0],
        search_outcome.optimal_values[1],
    );
    println!("scoregrid\n{}", scoregrid.verbose());

    let fitted_h2h = frame_prices(&scoregrid, &h2h.outcomes, &h2h.market.overround);
    println!("fitted_h2h: {fitted_h2h:?}");
    let table_h2h = print_market(&LabelledMarket {
        outcomes: h2h.outcomes,
        market: fitted_h2h,
    });
    println!("H2H:\n{}", Console::default().render(&table_h2h));

    let fitted_goals_ou = frame_prices(&scoregrid, &goals_ou.outcomes, &goals_ou.market.overround);
    let table_goals_ou = print_market(&LabelledMarket {
        outcomes: goals_ou.outcomes,
        market: fitted_goals_ou,
    });
    println!("Goals O/U:\n{}", Console::default().render(&table_goals_ou));

    let fitted_correct_score = frame_prices(&scoregrid, &correct_score.outcomes, &correct_score.market.overround);
    let table_correct_score = print_market(&LabelledMarket {
        outcomes: correct_score.outcomes,
        market: fitted_correct_score,
    });
    println!("Correct score:\n{}", Console::default().render(&table_correct_score));
}

fn fit_market(map: &HashMap<Outcome, f64>) -> LabelledMarket {
    let mut outcomes = Vec::with_capacity(map.len());
    let mut prices = Vec::with_capacity(map.len());
    for (outcome, &price) in map {
        outcomes.push(outcome.clone());
        prices.push(price);
    }
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
            max_steps: 10,
            acceptable_residual: 1e-9,
            bounds: Capture::Owned(vec![0.0..=0.5, 0.0..=0.5]),
            resolution: 10,
        },
        |values| {
            let scoregrid = create_scoregrid(values[0], values[1]);

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

fn create_scoregrid(interval_home_prob: f64, interval_away_prob: f64) -> Matrix<f64> {
    const INTERVALS: usize = 6;
    let space = ScoreOutcomeSpace {
        interval_home_prob,
        interval_away_prob,
    };
    let mut fixtures = IterFixtures::new(INTERVALS);
    let iter = Iter::new(&space, &mut fixtures);
    let mut scoregrid = Matrix::allocate(INTERVALS + 1, INTERVALS + 1);
    scoregrid::from_iterator(iter, &mut scoregrid);
    scoregrid
}

fn frame_prices(scoregrid: &Matrix<f64>, outcomes: &[Outcome], overround: &Overround) -> Market {
    let probs = outcomes
        .iter()
        .map(|outcome| outcome.gather(scoregrid))
        .collect::<Vec<_>>();
    Market::frame(overround, probs)
}

pub fn print_market(market: &LabelledMarket) -> Table {
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
