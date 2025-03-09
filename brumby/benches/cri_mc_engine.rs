use brumby::capture::{Capture, CaptureMut};
use brumby::dilative::DilatedProbs;
use brumby::mc::MonteCarloEngine;
use brumby::probs::SliceExt;
use brumby::selection::{Rank, Runner, Selection};
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

    probs.normalise(1.0);
    let mut podium = [usize::MAX; 4];
    let mut bitmap = [true; 14];
    let mut totals = [1.0; 4];
    let mut engine = MonteCarloEngine::default()
        .with_trials(1_000)
        .with_bitmap(CaptureMut::Borrowed(&mut bitmap))
        .with_totals(CaptureMut::Borrowed(&mut totals))
        .with_podium(CaptureMut::Borrowed(&mut podium))
        .with_probs(Capture::Owned(
            DilatedProbs::default()
                .with_win_probs(Capture::Borrowed(&probs))
                .with_podium_places(4)
                .into(),
        ));

    {
        // sanity check
        let selections = [
            Selection::Span {
                runner: Runner::number(1),
                ranks: Rank::first()..=Rank::number(1),
            },
            Selection::Span {
                runner: Runner::number(2),
                ranks: Rank::first()..=Rank::number(2),
            },
        ];
        let frac = engine.simulate(&selections);
        assert!(frac.numerator > 0);
        assert_eq!(1_000, frac.denominator);
    }

    c.bench_function("cri_mc_engine_exacta_1k", |b| {
        let selections = [
            Selection::Span {
                runner: Runner::number(1),
                ranks: Rank::first()..=Rank::number(1),
            },
            Selection::Span {
                runner: Runner::number(2),
                ranks: Rank::first()..=Rank::number(2),
            },
        ];
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
    c.bench_function("cri_mc_engine_trifecta_1k", |b| {
        let selections = [
            Selection::Span {
                runner: Runner::number(1),
                ranks: Rank::first()..=Rank::number(1),
            },
            Selection::Span {
                runner: Runner::number(2),
                ranks: Rank::first()..=Rank::number(2),
            },
            Selection::Span {
                runner: Runner::number(3),
                ranks: Rank::first()..=Rank::number(3),
            },
        ];
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
    c.bench_function("cri_mc_engine_first4_1k", |b| {
        let selections = [
            Selection::Span {
                runner: Runner::number(1),
                ranks: Rank::first()..=Rank::number(1),
            },
            Selection::Span {
                runner: Runner::number(2),
                ranks: Rank::first()..=Rank::number(2),
            },
            Selection::Span {
                runner: Runner::number(3),
                ranks: Rank::first()..=Rank::number(3),
            },
            Selection::Span {
                runner: Runner::number(4),
                ranks: Rank::first()..=Rank::number(4),
            },
        ];
        b.iter(|| {
            engine.simulate(&selections);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
