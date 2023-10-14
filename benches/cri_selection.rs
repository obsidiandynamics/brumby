use bentobox::selection::Selection;
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let podium = [10, 20, 30, 40];
    {
        let selection = Selection::Top { runner: 10, rank: 0 };
        assert!(selection.matches(&podium));
        c.bench_function("cri_selection_top1", |b| {
            b.iter(|| selection.matches(&podium));
        });
    }
    {
        let selection = Selection::Top { runner: 20, rank: 1 };
        assert!(selection.matches(&podium));
        c.bench_function("cri_selection_top2", |b| {
            b.iter(|| selection.matches(&podium));
        });
    }
    {
        let selection = Selection::Top { runner: 30, rank: 2 };
        assert!(selection.matches(&podium));
        c.bench_function("cri_selection_top3", |b| {
            b.iter(|| selection.matches(&podium));
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
