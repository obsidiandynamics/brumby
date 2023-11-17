use criterion::{Criterion, criterion_group, criterion_main};

use brumby::interval;
use brumby::interval::IntervalConfig;

fn criterion_benchmark(c: &mut Criterion) {
    fn run(intervals: u8) -> usize {
        interval::explore(&IntervalConfig {
            intervals,
            home_prob: 0.25,
            away_prob: 0.25,
            common_prob: 0.25,
            max_total_goals: u16::MAX,
            scorers: vec![],
        }).prospects.len()
    }

    // sanity check
    assert_eq!(16, run(3));

    c.bench_function("cri_interval_90", |b| {
        b.iter(|| {
            run(90)
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
