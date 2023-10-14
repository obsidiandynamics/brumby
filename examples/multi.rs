use tinyrand::StdRand;
use bentobox::mc;
use bentobox::probs::VecExt;

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
    println!("probs: {probs:?}");
    println!("overround: {overround:.3}");
    let mut podium = vec![usize::MAX; 4];
    let mut bitmap = vec![false; probs.len()];
    let mut rand = StdRand::default();
    mc::run_once(&probs, &mut podium, &mut bitmap, &mut rand);
}
