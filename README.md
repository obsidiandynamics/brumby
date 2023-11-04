`brumby`
===
A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events. Derives probabilities for placing in arbitrary positions given only win probabilities. Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

[![Crates.io](https://img.shields.io/crates/v/brumby?style=flat-square&logo=rust)](https://crates.io/crates/brumby)
[![docs.rs](https://img.shields.io/badge/docs.rs-brumby-blue?style=flat-square&logo=docs.rs)](https://docs.rs/brumby)
[![Build Status](https://img.shields.io/github/actions/workflow/status/obsidiandynamics/brumby/master.yml?branch=master&style=flat-square&logo=github)](https://github.com/obsidiandynamics/brumby/actions/workflows/master.yml)

# Performance
Circa 15M simulations/sec of a top-4 podium over 14 runners using the [tinyrand](https://github.com/obsidiandynamics/tinyrand) RNG. (Per thread, benchmarked on Apple M2 Pro.) Roughly 70% of time is spent in the RNG routine.

# Example
Sourced from `examples/multi.rs`. To try this example, run `just multi` on the command line. You'll need [just](https://github.com/casey/just) installed.

```rust
use std::error::Error;
use std::path::PathBuf;

use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

use brumby::display::DisplaySlice;
use brumby::file::ReadJsonFile;
use brumby::market::{Market, OverroundMethod};
use brumby::model::{Calibrator, Config, WinPlace};
use brumby::model::cf::Coefficients;
use brumby::model::fit::FitOptions;
use brumby::print;
use brumby::selection::{Rank, Runner};

fn main() -> Result<(), Box<dyn Error>> {
    // prices taken from a popular website
    let win_prices = vec![
        1.65,
        7.0,
        15.0,
        9.5,
        f64::INFINITY, // a scratched runner
        9.0,
        7.0,
        11.0,
        151.0,
    ];
    let place_prices = vec![
        1.12,
        1.94,
        3.2,
        2.3,
        f64::INFINITY, // a scratched runner
        2.25,
        1.95,
        2.55,
        28.0,
    ];

    // load coefficients from a file and create a calibrator for model fitting
    let coefficients = Coefficients::read_json_file(PathBuf::from("config/thoroughbred.cf.json"))?;
    let config = Config {
        coefficients,
        fit_options: FitOptions::fast()
    };
    let calibrator = Calibrator::try_from(config)?;

    // fit Win and Place probabilities from the supplied prices, undoing the overrounds
    let wp_markets = WinPlace {
        win: Market::fit(&OverroundMethod::Multiplicative, win_prices, 1.),
        place: Market::fit(&OverroundMethod::Multiplicative, place_prices, 3.),
        places_paying: 3,
    };

    // we have overrounds for Win and Place; extrapolate for Top-2 and Top-4 markets
    let overrounds = wp_markets.extrapolate_overrounds()?;

    // fit a model using the Win/Place prices and extrapolated overrounds
    let model = calibrator.fit(wp_markets, &overrounds)?.value;
    
    // nicely format the derived price matrix
    let table = print::tabulate_derived_prices(&model.top_n.as_price_matrix());
    println!("\n{}", Console::default().render(&table));
    
    // simulate a same-race multi for a chosen selection vector using the previously fitted model
    let selections = vec![
        Runner::number(6).top(Rank::number(1)),
        Runner::number(7).top(Rank::number(2)),
        Runner::number(8).top(Rank::number(3)),
    ];
    let multi_price = model.derive_multi(&selections)?.value;
    println!(
        "{} with probability {:.6} is priced at {:.2}",
        DisplaySlice::from(&*selections),
        multi_price.probability,
        multi_price.price
    );

    Ok(())
}
```