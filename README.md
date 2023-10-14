`bentobox`
===
A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events. Derives probabilities for placing in arbitrary positions given only win probabilities. Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

# Performance
Circa 20M simulations/sec of a top-4 podium over 14 runners using the [tinyrand](https://github.com/obsidiandynamics/tinyrand) RNG. Roughly 80% of time is spent in the RNG routine.

# Example
Sourced from `examples/multi.rs`.

```rust
use bentobox::mc;
use bentobox::probs::SliceExt;
use bentobox::selection::Selection;
use tinyrand::StdRand;
use bentobox::capture::{CaptureMut, Capture};

// probs taken from a popular website
let mut probs = vec![
    1.0 / 11.0,
    1.0 / 41.0,
    1.0 / 18.0,
    1.0 / 12.0,
    1.0 / 91.0,
    1.0 / 101.0,
    1.0 / 4.8,
    1.0 / 14.0,
    1.0 / 2.9,
    1.0 / 91.0,
    1.0 / 9.0,
    1.0 / 91.0,
    1.0 / 5.0,
    1.0 / 21.0,
];

// force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
let overround = probs.normalize();

println!("fair probs: {probs:?}");
println!("overround: {overround:.3}");

// create an MC engine for reuse
let mut engine = mc::MonteCarloEngine::default()
    .with_iterations(10_000)
    .with_probabilities(Capture::Borrowed(&probs))
    .with_podium_places(4)
    .with_rand(CaptureMut::Owned(StdRand::default()));

// simulate top-N rankings for all runners
// NOTE: rankings and runner numbers are zero-based
for runner in 0..probs.len() {
    println!("runner: {runner}");
    for rank in 0..4 {
        let frac = engine.simulate(&vec![Selection::Top { runner, rank }]);
        println!(
            "    rank: 0~{rank}, prob: {}, fair price: {:.3}, market odds: {:.3}",
            frac.quotient(),
            1.0 / frac.quotient(),
            1.0 / frac.quotient() / overround
        );
    }
}

// simulate a same-race multi for a chosen selection vector
let selections = vec![
    Selection::Top { runner: 0, rank: 0 },
    Selection::Top { runner: 1, rank: 1 },
    Selection::Top { runner: 2, rank: 2 },
];
let frac = engine.simulate(&selections);
println!(
    "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
    frac.quotient(),
    1.0 / frac.quotient(),
    1.0 / frac.quotient() / overround
);
```