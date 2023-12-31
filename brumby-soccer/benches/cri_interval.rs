use criterion::{criterion_group, criterion_main, Criterion};
use brumby::sv;

use brumby_soccer::interval;
use brumby_soccer::interval::{Config, PruneThresholds, BivariateProbs, TeamProbs, UnivariateProbs};

fn criterion_benchmark(c: &mut Criterion) {
    fn run(intervals: u8, max_total_goals: u16) -> usize {
        interval::explore(
            &Config {
                intervals,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    assists: UnivariateProbs { home: 1.0, away: 1.0 },
                },
                player_probs: sv![],
                prune_thresholds: PruneThresholds {
                    max_total_goals,
                    min_prob: 1e-6,
                },
                expansions: Default::default(),
            },
            0..intervals,
        )
        .prospects
        .len()
    }

    // sanity check
    assert_eq!(81, run(4, u16::MAX));

    c.bench_function("cri_interval_18_min_1e-6", |b| {
        b.iter(|| run(18, u16::MAX));
    });

    c.bench_function("cri_interval_36_min_1e-6", |b| {
        b.iter(|| run(36, u16::MAX));
    });

    c.bench_function("cri_interval_90_min_1e-6_max_8_goals", |b| {
        b.iter(|| run(90, 8));
    });

    c.bench_function("cri_interval_90_min_1e-6_max_16_goals", |b| {
        b.iter(|| run(90, 16));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
