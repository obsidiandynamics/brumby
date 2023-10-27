use anyhow::bail;
use clap::Parser;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};
use std::error::Error;
use std::ops::{Deref, Range, RangeInclusive};
use std::path::PathBuf;

use bentobox::capture::Capture;
use bentobox::data::{download_by_id, read_from_file, EventDetailExt, RaceSummary};
use bentobox::linear::Matrix;
use bentobox::mc::DilatedProbs;
use bentobox::opt::{gd, GradientDescentConfig, GradientDescentOutcome};
use bentobox::print::{tabulate_derived_prices, DerivedPrice, tabulate_values, tabulate_probs, tabulate_prices};
use bentobox::probs::{MarketPrice, SliceExt};
use bentobox::selection::{Rank, Runner, Selection, Selections};
use bentobox::{mc, overround};
use bentobox::display::DisplaySlice;

const MC_ITERATIONS_TRAIN: u64 = 100_000;
const MC_ITERATIONS_EVAL: u64 = 1_000_000;
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
    let args = Args::parse();
    args.validate()?;
    println!("args: {args:?}");

    let race = read_race_data(&args).await?;
    println!("prices= {}", race.prices.verbose());
    let mut win_probs: Vec<_> = race
        .prices
        .row_slice(0)
        .iter()
        .map(|price| 1.0 / price)
        .collect();
    let place_prices = race.prices.row_slice(2).to_vec();

    let win_overround = win_probs.normalise(1.0);
    let mut place_probs: Vec<_> = place_prices.invert().collect();
    let place_overround = place_probs.normalise(3.0) / 3.0;
    // let outcome = fit_holistic(&win_probs, &place_prices);
    //TODO skipping the fit for now
    // let outcome = GradientDescentOutcome {
    //     iterations: 0,
    //     optimal_residual: 0.008487581502095446,
    //     optimal_value: 0.12547299468220757,
    // };
    let outcome = GradientDescentOutcome {
        iterations: 0,
        optimal_residual: 0.0,
        optimal_value: 0.0,
    };
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
        .with_dilatives(Capture::Borrowed(&dilatives))
        .into();

    let mut scenarios = Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            scenarios[(rank, runner)] = vec![Runner::index(runner).top(Rank::index(rank))].into();
        }
    }

    // let overround_step = (win_overround - place_overround) / 2.0;
    // let ranked_overrounds = vec![
    //     win_overround,
    //     win_overround - overround_step,
    //     place_overround,
    //     place_overround - overround_step,
    // ];
    // let overround_step = (win_overround - place_overround) / 2.0;
    let ranked_overrounds = vec![
        win_overround,
        race.prices.row_slice(1).invert().sum::<f64>() / 2.0,
        place_overround,
        race.prices.row_slice(3).invert().sum::<f64>() / 4.0,
    ];

    // let individual_fit_outcome = fit_individual(
    //     &scenarios,
    //     &dilated_probs,
    //     2,
    //     1..=3,
    //     ranked_overrounds[2],
    //     race.prices.row_slice(2),
    // );
    // println!(
    //     "individual fitting complete: best_msre: {}, RMSRE: {}",
    //     individual_fit_outcome.optimal_msre,
    //     individual_fit_outcome.optimal_msre.sqrt()
    // );
    // println!("fitted probs:\n{}", individual_fit_outcome.optimal_probs.verbose());
    // let probs = individual_fit_outcome.optimal_probs;

    let mut probs = dilated_probs;
    for rank in 0..4 {
        let individual_fit_outcome = fit_individual(
            &scenarios,
            &probs,
            rank,
            rank..=rank,
            ranked_overrounds[rank],
            race.prices.row_slice(rank),
        );
        println!(
            "individual fitting complete for {}: best_msre: {}, RMSRE: {}",
            Rank::index(rank),
            individual_fit_outcome.optimal_msre,
            individual_fit_outcome.optimal_msre.sqrt()
        );
        // println!("fitted probs:\n{}", individual_fit_outcome.optimal_probs.verbose());
        println!("Fitted probs");
        let probs_table = tabulate_probs(&individual_fit_outcome.optimal_probs);
        println!("{}", Console::default().render(&probs_table));
        probs = individual_fit_outcome.optimal_probs;
    }
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(MC_ITERATIONS_TRAIN)
        .with_probs(probs.into());
    // engine.set_probs(individual_fit_outcome.optimal_probs.into());

    let mut counts = Matrix::allocate(podium_places, num_runners);
    engine.set_iterations(MC_ITERATIONS_EVAL);
    engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
    let mut derived_prices = Matrix::allocate(podium_places, num_runners);
    for runner in 0..num_runners {
        for rank in 0..podium_places {
            let probability = counts[(rank, runner)] as f64 / engine.iterations() as f64;
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
    let table = tabulate_derived_prices(&derived_prices);
    println!("{}", Console::default().render(&table));

    let errors: Vec<_> = (0..podium_places).map(|rank| {
        compute_msre(
            race.prices.row_slice(rank),
            derived_prices.row_slice(rank),
            &FITTED_PRICE_RANGES[rank],
        ).sqrt()
    }).collect();

    let dilatives_table = tabulate_values(&dilatives, "Dilative");
    let errors_table = tabulate_values(&errors, "RMSRE");
    let overrounds_table = tabulate_values(&ranked_overrounds, "Overround");
    let sample_prices_table = tabulate_prices(&race.prices);
    let summary_table = Table::with_styles(Styles::default().with(HAlign::Centred))
        .with_cols(vec![
            Col::default(),
            Col::new(Styles::default().with(Separator(true)).with(MinWidth(5))),
            Col::default(),
            Col::new(Styles::default().with(Separator(true)).with(MinWidth(5))),
            Col::default(),
            Col::new(Styles::default().with(Separator(true)).with(MinWidth(5))),
            Col::default()
        ])
        .with_row(Row::from(["Initial dilatives", "", "Errors", "", "Overrounds", "", "Sample prices"]))
        .with_row(Row::new(Styles::default(), vec![
            dilatives_table.into(),
            "".into(),
            errors_table.into(),
            "".into(),
            overrounds_table.into(),
            "".into(),
            sample_prices_table.into()
        ]));
    println!("{}", Console::default().render(&summary_table));

    if let Some(selections) = args.selections {
        // let overround = win_overround.powi(selections.len() as i32);
        let mut overround = 1.0;
        for selection in &*selections {
            match selection {
                Selection::Span { ranks, .. } => {
                    overround *= ranked_overrounds[ranks.end().as_index()];
                }
                Selection::Exact { rank, .. } => {
                    overround *= ranked_overrounds[rank.as_index()];
                }
            }
        }
        let frac = engine.simulate(&selections);
        println!(
            "probability of {}: {}, fair price: {:.3}, overround: {overround:3}, market odds: {:.3}",
            DisplaySlice::from(&*selections),
            frac.quotient(),
            1.0 / frac.quotient(),
            overround::apply_with_cap(
                1.0 / frac.quotient(),
                overround
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
        return Ok(event_detail.summarise());
    }
    unreachable!()
}

fn fit_holistic(win_probs: &[f64], place_prices: &[f64]) -> GradientDescentOutcome {
    let mut place_probs: Vec<_> = place_prices.invert().collect();
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
                    scenarios[(rank, runner)] =
                        vec![Runner::index(runner).top(Rank::index(rank))].into();
                }
            }

            let dilated_probs: Matrix<_> = DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(win_probs))
                .with_dilatives(dilatives.into())
                .into();
            let mut engine = mc::MonteCarloEngine::default()
                .with_iterations(MC_ITERATIONS_TRAIN)
                .with_probs(dilated_probs.into());
            let mut counts = Matrix::allocate(podium_places, num_runners);
            engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());

            let mut derived_prices = vec![0.0; num_runners];
            for runner in 0..num_runners {
                let derived_prob = counts[(2, runner)] as f64 / engine.iterations() as f64;
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

struct IndividualFitOutcome {
    optimal_msre: f64,
    optimal_probs: Matrix<f64>,
}

fn fit_individual(
    scenarios: &Matrix<Selections>,
    dilated_probs: &Matrix<f64>,
    rank: usize,
    adj_ranks: RangeInclusive<usize>,
    overround: f64,
    sample_prices: &[f64],
) -> IndividualFitOutcome {
    let podium_places = dilated_probs.rows();
    let num_runners = dilated_probs.cols();
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(MC_ITERATIONS_TRAIN)
        .with_probs(Capture::Borrowed(&dilated_probs));

    let mut optimal_msre = f64::MAX;
    let mut optimal_probs = Matrix::empty();
    for round in 0..100 {
        println!("INDIVIDUAL FITTING round {round}");
        let mut counts = Matrix::allocate(podium_places, num_runners);
        engine.simulate_batch(scenarios.flatten(), counts.flatten_mut());
        let mut derived_prices = Matrix::allocate(podium_places, num_runners);
        for runner in 0..num_runners {
            for rank in 0..podium_places {
                let probability = counts[(rank, runner)] as f64 / engine.iterations() as f64;
                let fair_price = 1.0 / probability;
                let market_price = overround::apply_with_cap(fair_price, overround);
                derived_prices[(rank, runner)] = market_price;
            }
        }
        let fitted_prices = derived_prices.row_slice(rank);
        println!("fitted prices:  {fitted_prices:?}");
        println!("sample prices: {sample_prices:?}");
        let msre = compute_msre(sample_prices, fitted_prices, &FITTED_PRICE_RANGES[rank]);
        println!("msre: {msre}, rmsre: {}", msre.sqrt());

        let mut current_probs = engine.probs().unwrap().deref().clone();
        if msre < optimal_msre {
            optimal_msre = msre;
            optimal_probs = current_probs.clone();
        } else if msre < TARGET_MSRE {
            break;
        }

        // let mut adjustments = vec![0.0; place_prices.len()];
        for (runner, sample_price) in sample_prices.iter().enumerate() {
            if sample_price.is_finite() {
                let fitted_price = fitted_prices[runner];
                let adj = fitted_price / sample_price;
                // adjustments[runner] = adj;
                for rank in adj_ranks.clone() {
                    scale_prob_capped(&mut current_probs[(rank, runner)], adj);
                }
            };
        }
        for rank in adj_ranks.clone() {
            current_probs.row_slice_mut(rank).normalise(1.0);
        }
        // println!("adjustments: {adjustments:?}");
        println!("adjusted probs: {:?}", current_probs.row_slice(rank));
        engine.reset_rand();
        engine.set_probs(current_probs.into());
    }

    IndividualFitOutcome {
        optimal_probs,
        optimal_msre,
    }
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
