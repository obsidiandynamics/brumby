use std::error::Error;
use std::ops::Range;
use std::path::{PathBuf};
use anyhow::bail;
use clap::Parser;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

use bentobox::capture::Capture;
use bentobox::linear::Matrix;
use bentobox::{mc, overround};
use bentobox::data::{EventDetailExt, RaceSummary, read_from_file};
use bentobox::mc::DilatedProbs;
use bentobox::opt::{gd, GradientDescentConfig, GradientDescentOutcome};
use bentobox::print::{DerivedPrice, tabulate};
use bentobox::probs::SliceExt;
use bentobox::selection::{Runner, Selection, Selections};

const MC_ITERATIONS: u64 = 100_000;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// selections to price
    #[clap(short = 's', long)]
    selections: Option<Selections<'static>>,

    /// file to source the race data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// URL to source the race data from
    #[clap(short = 'u', long)]
    url: Option<String>
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        if self.file.is_none() && self.url.is_none() || self.file.is_some() && self.url.is_some(){
            bail!("either the -f or the -u flag must be specified");
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>>{
    let args = Args::parse();
    args.validate()?;
    println!("args: {args:?}");

    let race = read_race_data(&args)?;
    println!("prices= {}", race.prices.verbose());
    let mut win_probs: Vec<_> = race.prices.row_slice(0).iter().map(|price| 1.0 / price).collect();
    let place_prices = race.prices.row_slice(2).to_vec();

    // let mut win_probs = vec![
    //     1.0 / 1.55,
    //     1.0 / 12.0,
    //     1.0 / 6.50,
    //     1.0 / 9.00,
    //     1.0 / 9.00,
    //     1.0 / 61.0,
    //     1.0 / 7.5,
    //     1.0 / 81.0,
    // ];
    // let place_prices = vec![
    //     1.09,
    //     2.55,
    //     1.76,
    //     2.15,
    //     2.10,
    //     10.5,
    //     1.93,
    //     13.5
    // ];

    let win_overround = win_probs.normalise(1.0);
    let mut place_probs: Vec<_> = place_prices.iter().map(|odds| 1.0 / odds).collect();
    let place_overround = place_probs.normalise(3.0) / 3.0;
    let outcome = fit(&win_probs, &place_prices);
    println!("gradient descent outcome: {outcome:?}, RMSRE: {}", outcome.optimal_residual.sqrt());

    let dilatives = vec![0.0, outcome.optimal_value, outcome.optimal_value, outcome.optimal_value];
    let podium_places = dilatives.len();
    let num_runners = win_probs.len();
    let dilated_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(win_probs.into())
        .with_dilatives(dilatives.into())
        .into();
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(MC_ITERATIONS)
        .with_probs(dilated_probs.into());

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

    let overround_step = (win_overround - place_overround) / 2.0;
    let ranked_overrounds = vec![win_overround, win_overround - overround_step, place_overround, place_overround - overround_step];
    // {
    //     let mut counts = Matrix::allocate(podium_places, num_runners);
    //     engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
    //     let mut derived_prices = Matrix::allocate(podium_places, num_runners);
    //     for runner in 0..num_runners {
    //         for rank in 0..podium_places {
    //             let probability = counts[(rank, runner)] as f64 / MC_ITERATIONS as f64;
    //             let fair_price = 1.0 / probability;
    //             let market_price = overround::apply_with_cap(fair_price, ranked_overrounds[rank]);
    //             derived_prices[(rank, runner)] = market_price;
    //         }
    //     }
    //     println!("fitted prices:  {:?}", derived_prices.row_slice(2));
    //     println!("sample prices: {:?}", place_prices);
    // }

    let mut counts = Matrix::allocate(podium_places, num_runners);
    engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
    let mut derived_prices = Matrix::allocate(podium_places, num_runners);
    println!("ranked overrounds: {ranked_overrounds:?}");
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            let probability = counts[(rank, runner)] as f64 / MC_ITERATIONS as f64;
            let fair_price = 1.0 / probability;
            let market_price = overround::apply_with_cap(fair_price, ranked_overrounds[rank]);
            let price = DerivedPrice {
                probability,
                fair_price,
                market_price,
            };
            derived_prices[(rank, runner)] = price;
        }
    }
    let table = tabulate(&derived_prices);
    println!("{}", Console::default().render(&table));

    if let Some(selections) = args.selections {
        let frac = engine.simulate(&selections);
        println!(
            "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
            frac.quotient(),
            1.0 / frac.quotient(),
            overround::apply_with_cap(1.0 / frac.quotient(), win_overround.powi(selections.len() as i32))
        );
    }

    Ok(())
}

fn read_race_data(args: &Args) -> anyhow::Result<RaceSummary> {
    if let Some(path) = args.file.as_ref() {
        let event_detail = read_from_file(path)?;
        return Ok(event_detail.summarise());
    }

    unreachable!()
}

fn fit(win_probs: &[f64], place_prices: &[f64]) -> GradientDescentOutcome {
    let mut place_probs: Vec<_> = place_prices.iter().map(|odds| 1.0 / odds).collect();
    let place_overround = place_probs.normalise(3.0) / 3.0;

    const FITTED_PRICE_RANGE: Range<f64> = 1.0..50.0;

    gd(GradientDescentConfig {
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
            .with_win_probs(Capture::Borrowed(win_probs))
            .with_dilatives(dilatives.into())
            .into();
        let mut engine = mc::MonteCarloEngine::default()
            .with_iterations(MC_ITERATIONS)
            .with_probs(dilated_probs.into());
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

        let mut sq_error = 0.0;
        let mut derived_prices = vec![0.0; num_runners];
        for runner in 0..num_runners {
            let sample_price = place_prices[runner];
            if FITTED_PRICE_RANGE.contains(&sample_price) {
                let derived_prob = counts[(2, runner)] as f64 / MC_ITERATIONS as f64;
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
    })
}