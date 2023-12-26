use brumby_soccer::domain::{OfferType, OutcomeType, Player, Side};
use criterion::{criterion_group, criterion_main, Criterion};
use brumby::sv;

use brumby_soccer::interval::{explore, Exploration, Config, BivariateProbs, PruneThresholds, PlayerProbs, TeamProbs, UnivariateProbs};
use brumby_soccer::interval::query::isolate;

fn criterion_benchmark(c: &mut Criterion) {
    let player = Player::Named(Side::Home, "Markos".into());
    fn prepare(intervals: u8, max_total_goals: u16, player: Player) -> Exploration {
        explore(
            &Config {
                intervals,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    assists: UnivariateProbs { home: 1.0, away: 1.0 },
                },
                player_probs: sv![(player, PlayerProbs { goal: Some(0.25), assist: None })],
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

    c.bench_function("cri_isolate_first_goalscorer_18", |b| {
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
