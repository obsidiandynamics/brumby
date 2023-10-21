use bentobox::capture::Capture;
use bentobox::mc;
use bentobox::mc::DilatedProbs;
use bentobox::probs::SliceExt;
use criterion::{criterion_group, criterion_main, Criterion};
use tinyrand::{StdRand, Wyrand};
use tinyrand_alloc::Mock;

fn criterion_benchmark(c: &mut Criterion) {
    let mut probs = [
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
    probs.normalise(1.0);
    let probs = DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&probs))
        .undilated(4)
        .into();
    let mut podium = [usize::MAX; 4];
    let mut bitmap = [true; 14];

    // sanity check
    mc::run_once(&probs, &mut podium, &mut bitmap, &mut StdRand::default());
    for ranked_runner in podium {
        assert_ne!(usize::MAX, ranked_runner);
    }
    assert_eq!(4, bitmap.iter().filter(|&&flag| !flag).count());

    c.bench_function("cri_mc_wyrand", |b| {
        let mut rand = Wyrand::default();
        b.iter(|| {
            mc::run_once(&probs, &mut podium, &mut bitmap, &mut rand);
        });
    });

    c.bench_function("cri_mc_mock", |b| {
        let mut rand = Mock::default();
        b.iter(|| {
            mc::run_once(&probs, &mut podium, &mut bitmap, &mut rand);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
