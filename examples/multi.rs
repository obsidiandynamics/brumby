use bentobox::mc;
use bentobox::probs::VecExt;
use bentobox::selection::Selection;
use tinyrand::StdRand;

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
    let mut podium = vec![usize::MAX; 4];
    let mut bitmap = vec![false; probs.len()];
    let mut rand = StdRand::default();

    const ITERS: u64 = 1_000_000;

    // simulate top-N rankings for all runners
    for runner in 0..probs.len() {
        println!("runner: {runner}");
        for rank in 0..4 {
            let frac = mc::run_many(
                ITERS,
                &vec![Selection::Top { runner, rank }],
                &probs,
                &mut podium,
                &mut bitmap,
                &mut rand,
            );
            println!(
                "    rank: 0~{rank}, prob: {}, fair price: {:.3}, market odds: {:.3}",
                frac.dec(),
                1.0 / frac.dec(),
                1.0 / frac.dec() / overround
            );
        }
    }

    // simulate a 3-leg same-race multi
    let selections = vec![
        Selection::Top { runner: 0, rank: 0 },
        Selection::Top { runner: 1, rank: 1 },
        Selection::Top { runner: 2, rank: 2 },
    ];
    let frac = mc::run_many(
        ITERS,
        &selections,
        &probs,
        &mut podium,
        &mut bitmap,
        &mut rand,
    );
    println!(
        "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
        frac.dec(),
        1.0 / frac.dec(),
        1.0 / frac.dec() / overround
    );

    mc::run_once(&probs, &mut podium, &mut bitmap, &mut rand);
}
