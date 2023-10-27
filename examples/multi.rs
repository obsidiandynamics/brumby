use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

use bentobox::linear::Matrix;
use bentobox::{mc, overround};
use bentobox::mc::DilatedProbs;
use bentobox::print::{DerivedPrice, tabulate};
use bentobox::probs::SliceExt;
use bentobox::selection::{Rank, Runner};

fn main() {
    // probs taken from a popular website
    // let mut probs = vec![
    //     1.0 / 11.0,
    //     1.0 / 41.0,
    //     1.0 / 18.0,
    //     1.0 / 12.0,
    //     1.0 / 91.0,
    //     1.0 / 101.0,
    //     1.0 / 4.8,
    //     1.0 / 14.0,
    //     1.0 / 2.9,
    //     1.0 / 91.0,
    //     1.0 / 9.0,
    //     1.0 / 91.0,
    //     1.0 / 5.0,
    //     1.0 / 21.0,
    // ];
    // let mut probs = vec![
    //     1.0 / 2.0,
    //     1.0 / 12.0,
    //     1.0 / 3.0,
    //     1.0 / 9.50,
    //     1.0 / 7.50,
    //     1.0 / 126.0,
    //     1.0 / 23.0,
    //     1.0 / 14.0,
    // ];
    // let mut probs = vec![
    //     1.0 / 3.7,
    //     1.0 / 14.0,
    //     1.0 / 5.50,
    //     1.0 / 9.50,
    //     1.0 / 1.90,
    //     1.0 / 13.0,
    // ];
    let mut win_probs = vec![
        1.0 / 1.55,
        1.0 / 12.0,
        1.0 / 6.50,
        1.0 / 9.00,
        1.0 / 9.00,
        1.0 / 61.0,
        1.0 / 7.5,
        1.0 / 81.0,
    ];

    // force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
    let win_overround = win_probs.normalise(1.0);

    //fav-longshot bias removal
    // let favlong_dilate = -0.0;
    // win_probs.dilate_power(favlong_dilate);

    // let dilatives = [0.0, 0.20, 0.35, 0.5];
    // let dilatives = vec![0.0, 0.0, 0.0, 0.0];
    let dilatives = vec![0.0, 0.268, 0.067, 0.0];
    let podium_places = dilatives.len();
    let num_runners = win_probs.len();

    let ranked_overrounds = [win_overround, 1.239, 1.169, 1.12];

    println!("win probs: {win_probs:?}");
    println!("dilatives: {dilatives:?}");
    println!("overround: {win_overround:.3}");

    let dilated_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(win_probs.into())
        .with_dilatives(dilatives.into())
        .into();

    println!("rank-runner probabilities: \n{}", dilated_probs.verbose());

    // simulate top-N rankings for all runners
    // NOTE: rankings and runner numbers are zero-based
    let mut scenarios =
        Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            scenarios[(rank, runner)] = vec![Runner::index(runner).top(Rank::index(rank))]
            .into();
        }
    }

    // create an MC engine for reuse
    const ITERATIONS: u64 = 1_000_000;
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(ITERATIONS)
        .with_probs(dilated_probs.into());
    let mut counts = Matrix::allocate(podium_places, num_runners);
    engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

    let mut derived = Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            let probability = counts[(rank, runner)] as f64 / ITERATIONS as f64;
            let fair_price = 1.0 / probability;
            let market_price = overround::apply_with_cap(fair_price, ranked_overrounds[rank]);
            let price = DerivedPrice {
                probability,
                fair_price,
                market_price,
            };
            derived[(rank, runner)] = price;
        }
    }

    // for row in 0..derived.rows() {
    //     println!("sum for row {row}: {}", derived.row_slice(row).sum());
    //     // derived.row_slice_mut(row).normalise(row as f64 + 1.0);
    // }

    //fav-longshot bias addition
    // derived
    //     .row_slice_mut(0)
    //     .dilate_power(1.0 / (1.0 + favlong_dilate) - 1.0);

    let table = tabulate(&derived);
    println!("{}", Console::default().render(&table));

    // simulate a same-race multi for a chosen selection vector
    let selections = vec![
        Runner::number(1).top(Rank::number(1)),
        Runner::number(2).top(Rank::number(2)),
        Runner::number(3).top(Rank::number(3)),
    ];
    let frac = engine.simulate(&selections);
    println!(
        "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
        frac.quotient(),
        1.0 / frac.quotient(),
        overround::apply_with_cap(1.0 / frac.quotient(), win_overround.powi(selections.len() as i32))
    );
}
