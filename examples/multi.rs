use bentobox::mc;
use bentobox::probs::VecExt;
use bentobox::selection::Selection;
use tinyrand::StdRand;
use bentobox::capture::{CaptureMut, Capture};

fn main() {
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

    let overround = probs.normalize();
    println!("fair probs: {probs:?}");
    println!("overround: {overround:.3}");

    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(10_000)
        .with_probabilities(Capture::Borrowed(&probs))
        .with_podium_places(4)
        .with_rand(CaptureMut::Owned(StdRand::default()));

    // simulate top-N rankings for all runners
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
}
