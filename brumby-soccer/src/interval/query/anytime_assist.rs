use super::*;

#[inline]
#[must_use]
pub(crate) fn requirements() -> Expansions {
    Expansions {
        ht_score: false,
        ft_score: false,
        player_goal_stats: false,
        player_split_goal_stats: false,
        max_player_assists: 1,
        first_goalscorer: false,
    }
}

#[inline]
#[must_use]
pub(crate) fn prepare(outcome_type: &OutcomeType, player_lookup: &HashLookup<Player>) -> QuerySpec {
    match outcome_type {
        OutcomeType::Player(player) => {
            QuerySpec::PlayerLookup(player_lookup.index_of(player).unwrap())
        }
        OutcomeType::None => QuerySpec::NoAnytimeAssist,
        _ => panic!("{outcome_type:?} unsupported"),
    }
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PlayerLookup(target_player) => {
            let stats = &prospect.stats[*target_player];
            stats.assists > 0
        }
        QuerySpec::NoAnytimeAssist => !prospect.stats.iter().any(|stats| stats.assists > 0),
        _ => panic!("{query:?} unsupported"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Period, Score, Side};
    use crate::interval::{explore, IntervalConfig, PlayerProbs, BivariateProbs, TeamProbs};

    fn print_prospects(prospects: &Prospects) {
        for (prospect, prob) in prospects {
            println!("prospect: {prospect:?} @ {prob}");
        }
    }

    #[test]
    fn cannot_assist_to_self() {
        let alice = Player::Named(Side::Home, "Alice".into());
        let bob = Player::Named(Side::Home, "Bob".into());
        let exploration = explore(
            &IntervalConfig {
                intervals: 1,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs {
                        home: 0.25,
                        away: 0.25,
                        common: 0.25,
                    },
                    h2_goals: BivariateProbs {
                        home: 0.25,
                        away: 0.25,
                        common: 0.25,
                    },
                },
                player_probs: vec![
                    (
                        alice.clone(),
                        PlayerProbs {
                            goal: Some(0.25),
                            assist: Some(0.25),
                        },
                    ),
                    (
                        bob.clone(),
                        PlayerProbs {
                            goal: Some(0.4),
                            assist: Some(0.4),
                        },
                    ),
                ],
                prune_thresholds: Default::default(),
                expansions: Expansions {
                    ht_score: false,
                    ft_score: false,
                    player_goal_stats: true,
                    player_split_goal_stats: false,
                    max_player_assists: 1,
                    first_goalscorer: false,
                },
            },
            0..1,
        );
        print_prospects(&exploration.prospects);

        let alice_to_bob = isolate_set(
            &[
                (OfferType::AnytimeAssist, OutcomeType::Player(alice.clone())),
                (
                    OfferType::AnytimeGoalscorer,
                    OutcomeType::Player(bob.clone()),
                ),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!((0.25 + 0.25) * 0.4 * 0.25, alice_to_bob, "{alice_to_bob}");

        let bob_to_alice = isolate_set(
            &[
                (OfferType::AnytimeAssist, OutcomeType::Player(bob.clone())),
                (
                    OfferType::AnytimeGoalscorer,
                    OutcomeType::Player(alice.clone()),
                ),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!((0.25 + 0.25) * 0.25 * 0.4, bob_to_alice, "{bob_to_alice}");

        let alice_to_alice = isolate_set(
            &[
                (OfferType::AnytimeAssist, OutcomeType::Player(alice.clone())),
                (
                    OfferType::AnytimeGoalscorer,
                    OutcomeType::Player(alice.clone()),
                ),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!(0.0, alice_to_alice);
    }

    #[test]
    fn cannot_assist_across_sides() {
        let alice = Player::Named(Side::Home, "Alice".into());
        let bob = Player::Named(Side::Away, "Bob".into());
        let exploration = explore(
            &IntervalConfig {
                intervals: 1,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs {
                        home: 0.25,
                        away: 0.25,
                        common: 0.25,
                    },
                    h2_goals: BivariateProbs {
                        home: 0.25,
                        away: 0.25,
                        common: 0.25,
                    },
                },
                player_probs: vec![
                    (
                        alice.clone(),
                        PlayerProbs {
                            goal: Some(0.25),
                            assist: Some(0.25),
                        },
                    ),
                    (
                        bob.clone(),
                        PlayerProbs {
                            goal: Some(0.4),
                            assist: Some(0.4),
                        },
                    ),
                ],
                prune_thresholds: Default::default(),
                expansions: Expansions {
                    ht_score: false,
                    ft_score: true,
                    player_goal_stats: true,
                    player_split_goal_stats: false,
                    max_player_assists: 1,
                    first_goalscorer: false,
                },
            },
            0..1,
        );
        print_prospects(&exploration.prospects);

        let alice_to_bob = isolate_set(
            &[
                (OfferType::AnytimeAssist, OutcomeType::Player(alice.clone())),
                (
                    OfferType::AnytimeGoalscorer,
                    OutcomeType::Player(bob.clone()),
                ),
                // the third condition is necessary because if the score is 1:1, Alice could have assisted to Other while Bob also scored
                (OfferType::CorrectScore(Period::FullTime), OutcomeType::Score(Score { home: 1, away: 0 })),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!(0.0, alice_to_bob, "{alice_to_bob}");
    }
}
