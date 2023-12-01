use criterion::{Criterion, criterion_group, criterion_main};

use brumby::interval;
use brumby::interval::IntervalConfig;

fn criterion_benchmark(c: &mut Criterion) {
    fn run(intervals: u8, max_total_goals: u16) -> usize {
        interval::explore(&IntervalConfig {
            intervals,
            home_prob: 0.25,
            away_prob: 0.25,
            common_prob: 0.25,
            max_total_goals,
            players: vec![],
        }).prospects.len()
    }

    // sanity check
    assert_eq!(16, run(3, u16::MAX));

    c.bench_function("cri_interval_18_unbounded", |b| {
        b.iter(|| {
            run(18, u16::MAX)
        });
    });

    c.bench_function("cri_interval_90_unbounded", |b| {
        b.iter(|| {
            run(90, u16::MAX)
        });
    });

    c.bench_function("cri_interval_90_max_8", |b| {
        b.iter(|| {
            run(90, 8)
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
