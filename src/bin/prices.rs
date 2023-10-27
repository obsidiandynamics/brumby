use anyhow::bail;
use clap::Parser;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Col, Row, Table};
use std::error::Error;
use std::ops::{Deref, Range};
use std::path::PathBuf;

use bentobox::capture::Capture;
use bentobox::data::{read_from_file, EventDetailExt, RaceSummary, download_by_id};
use bentobox::linear::Matrix;
use bentobox::mc::DilatedProbs;
use bentobox::opt::{gd, GradientDescentConfig, GradientDescentOutcome};
use bentobox::print::{tabulate, DerivedPrice};
use bentobox::probs::{MarketPrice, SliceExt};
use bentobox::selection::{Rank, Runner, Selection, Selections};
use bentobox::{mc, overround};

const MC_ITERATIONS: u64 = 100_000;
const FITTED_PRICE_RANGES: [Range<f64>; 4] = [1.0..50.0, 1.0..15.0, 1.0..10.0, 1.0..5.0];
const TARGET_MSRE: f64 = 1e-6;

#[derive(Debug, clap::Parser, Clone)]
struct Args {
    /// selections to price
    #[clap(short = 's', long)]
    selections: Option<Selections<'static>>,

    /// file to source the race data from
    #[clap(short = 'f', long)]
    file: Option<PathBuf>,

    /// download race data by ID
    #[clap(short = 'd', long)]
    download: Option<u64>,
}
impl Args {
    fn validate(&self) -> anyhow::Result<()> {
        if self.file.is_none() && self.download.is_none() || self.file.is_some() && self.download.is_some() {
            bail!("either the -f or the -d flag must be specified");
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    args.validate()?;
    println!("args: {args:?}");

    let race = read_race_data(&args).await?;
    println!("prices= {}",race.prices.verbose());
    let mut win_probs: Vec<_> = race
        .prices
        .row_slice(0)
        .iter()
        .map(|price| 1.0 / price)
        .collect();
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
    //TODO skipping the fit for now
    // let outcome = GradientDescentOutcome {
    //     iterations: 0,
    //     optimal_residual: 0.008487581502095446,
    //     optimal_value: 0.12547299468220757,
    // };
    // let outcome = GradientDescentOutcome {
    //     iterations: 0,
    //     optimal_residual: 0.0,
    //     optimal_value: 0.0,
    // };
    println!(
        "gradient descent outcome: {outcome:?}, RMSRE: {}",
        outcome.optimal_residual.sqrt()
    );

    let dilatives = vec![
        0.0,
        outcome.optimal_value,
        outcome.optimal_value,
        outcome.optimal_value,
    ];
    let podium_places = dilatives.len();
    let num_runners = win_probs.len();
    let dilated_probs: Matrix<_> = DilatedProbs::default()
        .with_win_probs(win_probs.into())
        .with_dilatives(dilatives.into())
        .into();
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(MC_ITERATIONS)
        .with_probs(Capture::Borrowed(&dilated_probs));

    let mut scenarios = Matrix::allocate(podium_places, num_runners);
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
    let ranked_overrounds = vec![
        win_overround,
        win_overround - overround_step,
        place_overround,
        place_overround - overround_step,
    ];
    let mut best_msre = f64::MAX;
    let mut best_probs = Matrix::empty();
    for round in 0..100 {
        println!("INDIVIDUAL FITTING round {round}");
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
        let mut derived_prices = Matrix::allocate(podium_places, num_runners);
        for runner in 0..num_runners {
            for rank in 0..podium_places {
                let probability = counts[(rank, runner)] as f64 / MC_ITERATIONS as f64;
                let fair_price = 1.0 / probability;
                let market_price = overround::apply_with_cap(fair_price, ranked_overrounds[rank]);
                derived_prices[(rank, runner)] = market_price;
            }
        }
        let fitted_prices = derived_prices.row_slice(2);
        println!("fitted prices:  {fitted_prices:?}");
        println!("sample prices: {place_prices:?}");
        let msre = compute_msre(&place_prices, fitted_prices, &FITTED_PRICE_RANGES[2]);
        println!("msre: {msre}, rmsre: {}", msre.sqrt());

        let mut current_probs = engine.probs().unwrap().deref().clone();
        if msre < best_msre {
            best_msre = msre;
            best_probs = current_probs.clone();
        } else if msre < TARGET_MSRE {
            break;
        }

        // let mut adjustments = vec![0.0; place_prices.len()];
        for (runner, sample_price) in place_prices.iter().enumerate() {
            if sample_price.is_finite() {
                let fitted_price = fitted_prices[runner];
                let adj = fitted_price / sample_price;
                // adjustments[runner] = adj;
                scale_prob_capped(&mut current_probs[(1, runner)], adj);
                scale_prob_capped(&mut current_probs[(2, runner)], adj);
                scale_prob_capped(&mut current_probs[(3, runner)], adj);
            };
        }
        current_probs.row_slice_mut(1).normalise(1.0);
        current_probs.row_slice_mut(2).normalise(1.0);
        current_probs.row_slice_mut(3).normalise(1.0);
        // println!("adjustments: {adjustments:?}");
        println!("adjusted probs: {:?}", current_probs.row_slice(2));
        engine.reset_rand();
        engine.set_probs(current_probs.into());
    }

    println!(
        "individual fitting complete: best_msre: {best_msre}, RMSRE: {}",
        best_msre.sqrt()
    );
    println!("fitted probs:\n{}", best_probs.verbose());
    engine.reset_rand();
    engine.set_probs(best_probs.into());

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

    let mut errors_table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Centred)),
            Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Rank".into(), "RMSRE".into()],
        ));
    for rank in 0..podium_places {
        let msre = compute_msre(
            race.prices.row_slice(rank),
            derived_prices.row_slice(rank),
            &FITTED_PRICE_RANGES[rank],
        );
        let rmsre = msre.sqrt();
        errors_table.push_row(Row::new(
            Styles::default(),
            vec![
                format!("{}", Rank::index(rank)).into(),
                format!("{rmsre:.6}").into(),
            ],
        ));
    }
    println!("{}", Console::default().render(&errors_table));

    if let Some(selections) = args.selections {
        let frac = engine.simulate(&selections);
        println!(
            "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
            frac.quotient(),
            1.0 / frac.quotient(),
            overround::apply_with_cap(
                1.0 / frac.quotient(),
                win_overround.powi(selections.len() as i32)
            )
        );
    }

    Ok(())
}

#[inline(always)]
fn scale_prob_capped(prob: &mut f64, adj: f64) {
    let scaled = f64::max(0.0, f64::min(*prob * adj, 1.0));
    *prob = scaled
}

async fn read_race_data(args: &Args) -> anyhow::Result<RaceSummary> {
    if let Some(path) = args.file.as_ref() {
        let event_detail = read_from_file(path)?;
        return Ok(event_detail.summarise());
    }
    if let Some(&id) = args.download.as_ref() {
        let event_detail = download_by_id(id).await?;
        return Ok(event_detail.summarise())
    }

    unreachable!()
}

fn fit(win_probs: &[f64], place_prices: &[f64]) -> GradientDescentOutcome {
    let mut place_probs: Vec<_> = place_prices.iter().map(|odds| 1.0 / odds).collect();
    let place_overround = place_probs.normalise(3.0) / 3.0;

    gd(
        GradientDescentConfig {
            init_value: 0.0,
            step: 0.01,
            min_step: 0.001,
            max_steps: 100,
        },
        |value| {
            let dilatives = vec![0.0, value, value, 0.0];
            let podium_places = dilatives.len();
            let num_runners = win_probs.len();

            let mut scenarios = Matrix::allocate(podium_places, num_runners);
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

            let mut derived_prices = vec![0.0; num_runners];
            for runner in 0..num_runners {
                let derived_prob = counts[(2, runner)] as f64 / MC_ITERATIONS as f64;
                let derived_price = f64::max(1.04, 1.0 / derived_prob / place_overround);
                derived_prices[runner] = derived_price;
            }
            let msre = compute_msre(&place_prices, &derived_prices, &FITTED_PRICE_RANGES[2]);
            println!("dilative: {value}, msre: {msre}");
            println!("derived_prices: {derived_prices:?}");
            println!("sample prices:  {place_prices:?}");
            msre
        },
    )
}

fn compute_msre<P: MarketPrice>(
    sample_prices: &[f64],
    fitted_prices: &[P],
    price_range: &Range<f64>,
) -> f64 {
    let mut sq_rel_error = 0.0;
    let mut counted = 0;
    for (runner, sample_price) in sample_prices.iter().enumerate() {
        let fitted_price: f64 = fitted_prices[runner].decimal();
        if fitted_price.is_finite() && price_range.contains(&sample_price) {
            counted += 1;
            let relative_error = (sample_price - fitted_price) / sample_price;
            sq_rel_error += relative_error.powi(2);
        }
    }
    sq_rel_error / counted as f64
}
