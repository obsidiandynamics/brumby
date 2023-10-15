use bentobox::capture::Capture;
use bentobox::mc;
use bentobox::probs::SliceExt;
use bentobox::selection::Selection;

fn main() {
    // probs taken from a popular website
    let mut probs = vec![
        1.0 / 2.0,
        1.0 / 12.0,
        1.0 / 3.0,
        1.0 / 9.50,
        1.0 / 7.50,
        1.0 / 126.0,
        1.0 / 23.0,
        1.0 / 14.0,
    ];

    // force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
    let overround = probs.normalize();

    println!("fair probs: {probs:?}");
    println!("overround: {overround:.3}");

    // create an MC engine for reuse
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(100_000)
        .with_win_probs(Capture::Borrowed(&probs))
        .with_podium_places(4);

    // simulate top-N rankings for all runners
    // NOTE: rankings and runner numbers are zero-based
    for runner in 0..probs.len() {
        println!("runner: {runner}");
        for rank in 0..4 {
            let frac = engine.simulate(&vec![Selection::Span {
                runner,
                ranks: 0..rank + 1,
            }]);
            println!(
                "    rank: 0..={rank}, prob: {}, fair price: {:.3}, market odds: {:.3}",
                frac.quotient(),
                1.0 / frac.quotient(),
                1.0 / frac.quotient() / overround
            );
        }
    }

    // simulate a same-race multi for a chosen selection vector
    let selections = vec![
        Selection::Span {
            runner: 0,
            ranks: 0..1,
        },
        Selection::Span {
            runner: 1,
            ranks: 0..2,
        },
        Selection::Span {
            runner: 2,
            ranks: 0..3,
        },
    ];
    let frac = engine.simulate(&selections);
    println!(
        "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
        frac.quotient(),
        1.0 / frac.quotient(),
        1.0 / frac.quotient() / overround.powi(selections.len() as i32)
    );
}
