use brumby_soccer::domain::{OfferType, OutcomeType, Player, Side};
use criterion::{criterion_group, criterion_main, Criterion};

use brumby_soccer::interval::{explore, isolate, Exploration, IntervalConfig, ScoringProbs, PruneThresholds};

fn criterion_benchmark(c: &mut Criterion) {
    let player = Player::Named(Side::Home, "Markos".into());
    fn prepare(intervals: u8, max_total_goals: u16, player: Player) -> Exploration {
        explore(
            &IntervalConfig {
                intervals,
                h1_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
                h2_probs: ScoringProbs { home_prob: 0.25, away_prob: 0.25, common_prob: 0.25 },
                players: vec![(player, 0.25)],
                prune_thresholds: PruneThresholds {
                    max_total_goals,
                    min_prob: 1e-6,
                },
                expansions: Default::default(),
            },
            0..intervals,
        )
    }

    // sanity check
    let exploration = prepare(18, u16::MAX, player.clone());
    // println!("prospects: {}", exploration.prospects.len());
    let isolated = isolate(
        &OfferType::AnytimeGoalscorer,
        &OutcomeType::Player(player.clone()),
        &exploration.prospects,
        &exploration.player_lookup,
    );
    assert!(isolated > 0.0);

    c.bench_function("cri_isolate_first_goalscorer_18_unbounded", |b| {
        let exploration = prepare(18, u16::MAX, player.clone());
        b.iter(|| {
            isolate(
                &OfferType::AnytimeGoalscorer,
                &OutcomeType::Player(player.clone()),
                &exploration.prospects,
                &exploration.player_lookup,
            )
        });
    });

    c.bench_function("cri_isolate_first_goalscorer_90_max_8", |b| {
        let exploration = prepare(90, 8, player.clone());
        b.iter(|| {
            isolate(
                &OfferType::AnytimeGoalscorer,
                &OutcomeType::Player(player.clone()),
                &exploration.prospects,
                &exploration.player_lookup,
            )
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
