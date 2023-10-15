use bentobox::capture::{Capture, CaptureMut};
use bentobox::mc::MonteCarloEngine;
use bentobox::probs::SliceExt;
use bentobox::selection::Selection;
use criterion::{criterion_group, criterion_main, Criterion};

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

    probs.normalize();
    let mut podium = [usize::MAX; 4];
    let mut bitmap = [true; 14];
    let mut engine = MonteCarloEngine::default()
        .with_iterations(1_000)
        .with_podium_places(4)
        .with_bitmap(CaptureMut::Borrowed(&mut bitmap))
        .with_probabilities(Capture::Borrowed(&probs))
        .with_podium(CaptureMut::Borrowed(&mut podium));

    {
        // sanity check
        let selections = [
            Selection::Span {
                runner: 0,
                ranks: 0..1,
            },
            Selection::Span {
                runner: 1,
                ranks: 0..2,
            },
        ];
        let frac = engine.simulate(&selections);
        assert!(frac.numerator > 0);
        assert_eq!(1_000, frac.denominator);
    }

    c.bench_function("cri_mc_engine_exacta", |b| {
        let selections = [
            Selection::Span {
                runner: 0,
                ranks: 0..1,
            },
            Selection::Span {
                runner: 1,
                ranks: 0..2,
            },
        ];
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
    c.bench_function("cri_mc_engine_trifecta", |b| {
        let selections = [
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
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
    c.bench_function("cri_mc_engine_first4", |b| {
        let selections = [
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
            Selection::Span {
                runner: 3,
                ranks: 0..4,
            },
        ];
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
