use std::ops::Range;
use bentobox::capture::Capture;
use bentobox::linear::Matrix;
use bentobox::mc;
use bentobox::mc::DilatedProbs;
use bentobox::opt::{gd, GradientDescentConfig};
use bentobox::probs::SliceExt;
use bentobox::selection::{Runner, Selection};

fn main() {
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
    let place_prices = vec![
        1.09,
        2.55,
        1.76,
        2.15,
        2.10,
        10.5,
        1.93,
        13.5
    ];
    let mut place_probs: Vec<_> = place_prices.iter().map(|odds| 1.0 / odds).collect();

    // force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
    let _win_overround = win_probs.normalise(1.0);
    let place_overround = place_probs.normalise(3.0) / 3.0;

    const FITTED_PRICE_RANGE: Range<f64> = 1.0..50.0;

    let outcome = gd(GradientDescentConfig {
        init_value: 0.0,
        step: 0.01,
        min_step: 0.001,
        max_steps: 100,
    }, |value| {
        let dilatives = vec![0.0, value, value, 0.0];
        let podium_places = dilatives.len();
        let num_runners = win_probs.len();

        let mut scenarios =
            Matrix::allocate(podium_places, num_runners);
        for runner in 0..num_runners {
            for rank in 0..podium_places {
                scenarios[(rank, runner)] = vec![Selection::Span {
                    runner: Runner::index(runner),
                    ranks: 0..rank + 1,
                }]
                .into();
            }
        }

        let dilated_probs: Matrix<_> = DilatedProbs::default()
            .with_win_probs(Capture::Borrowed(&win_probs))
            .with_dilatives(dilatives.into())
            .into();

        const ITERATIONS: u64 = 100_000;
        let mut engine = mc::MonteCarloEngine::default()
            .with_iterations(ITERATIONS)
            .with_probs(dilated_probs.into());
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

        let mut sq_error = 0.0;
        let mut derived_prices = vec![0.0; num_runners];
        for runner in 0..num_runners {
            let sample_price = place_prices[runner];
            if FITTED_PRICE_RANGE.contains(&sample_price) {
                let derived_prob = counts[(2, runner)] as f64 / ITERATIONS as f64;
                let derived_price = f64::max(1.04, 1.0 / derived_prob / place_overround);
                derived_prices[runner] = derived_price;
                let relative_error = (sample_price - derived_price) / sample_price;
                sq_error += relative_error.powi(2);
            }
        }
        println!("dilative: {value}, sq_error: {sq_error}");
        println!("derived_prices: {derived_prices:?}");
        println!("sample prices:  {place_prices:?}");

        sq_error
    });
    println!("gradient descent outcome: {outcome:?}");
}