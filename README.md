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
        fit_options: FitOptions::fast() // use the default presents in production; fast presets are used for testing
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

# How it works
Brumby is based on a regression-fitted, weighted Monte Carlo (MC) simulation.

Ideally, one would start with a set of fair probabilities. But in less ideal scenarios, you might have prices with an overround applied; it is first necessary to remove the overround to obtain a set of fair probabilities. Brumby includes both fitting and framing support for several overround methods, including _multiplicative_, _power_ and _fractional_. Fitting and framing capabilities are complementary: by 'fitting' it is meant the removal of overrounds from an existing market offering; by 'framing' it is meant the application of overrounds to a set of fair probabilities to obtain a corresponding set of market prices.

We start with a naive MC model. It simulates a podium finish using the supplied set of win probabilities. The naive model takes input a vector of probabilities _P_ and runs a series of random trials using a seeded pseudo-RNG. _P<sub>j</sub>_ is the probability of runner _j_ finishing first. In each trial, a uniformly distributed random number is mapped to an interval proportional to the supplied probabilities. This yields the winner of that trial. In the same trial, we then eliminate the winner's interval and adjust the output range of the RNG accordingly. The subsequent random number points to the second place getter, and so forth, until the desired podium is established. The podium is recorded for that trial and the next trial begins. As trials progress, the system establishes a matrix of top-N placement probabilities for each runner. (Row per rank and column per runner.) We have observed that 100K trials are generally sufficient to populate a probabilities matrix with sufficient precision to be used in wagering applications, such as pricing derivatives. Accuracy, however, is another matter.

A minor digression: Brumby relies on a custom-built MC engine that utilises pooled memory buffers to avoid costly allocations — `malloc` and `free` syscalls. It also uses a custom 2D matrix data structure that flattens all rows into a contiguous vector, thereby avoiding memory sparsity and takes advantage of CPU-level caching to update and retrieve nearby data points. This provides a reasonably performant, allocation-free MC simulator, taking ~65 ns to perform one trial of a 4-place podium over a field of 14.

The model above is unaware of the systemic biases such as the favourite-longshot bias, nor does it realise that longshots are more likely to place than their win probability might imply. (Conversely, favourites are less likely to place.) We don't go into details here, suffice it to say that real-life competitors are motivated differently depending on their position in the race. A naive MC model assumes a field of runners with constant exertion and no motives other than the ambition to come first.

The naive model is clearly insufficient. The assumptions it makes are simply untrue in any competitive sport that we are aware of. While the accuracy may be palatable in a field of approximately equally-capable runners, it breaks down in fields of diverse runner strengths. Since the target applications mostly comprise the latter, we must modify the model to skew the probabilities of a runner coming in positions 2 through to 4 (assuming a four-place podium) without affecting their win probability. We introduce a fine-grained biasing layer capable of targeting individual runners in specific finishing position. Where the naive model uses a vector of probabilities _P_, the biased model uses a matrix _W_ — with one row per finishing rank. _W<sub>i,j</sub>_ is the relative probability of runner _j_ taking position _i_. When all rows are identical, the biased model behaves identically to the naive one.

The offline-fitted model does not cater to specific biases present in individual races and, crucially, it does not protect the operator of the model against _internal arbitrage_ opportunities. This

