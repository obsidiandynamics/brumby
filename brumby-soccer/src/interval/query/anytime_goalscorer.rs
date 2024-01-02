use super::*;

#[inline]
#[must_use]
pub(crate) fn requirements() -> Expansions {
    Expansions {
        ht_score: false,
        ft_score: false,
        max_player_goals: 1,
        player_split_goal_stats: false,
        max_player_assists: 0,
        first_goalscorer: false,
    }
}

#[inline]
#[must_use]
pub(crate) fn prepare(
    outcome: &Outcome,
    player_lookup: &HashLookup<Player>,
) -> QuerySpec {
    match outcome {
        Outcome::Player(player) => {
            QuerySpec::PlayerLookup(player_lookup.index_of(player).unwrap())
        }
        Outcome::None => QuerySpec::NoAnytimeGoalscorer,
        _ => panic!("{outcome:?} unsupported"),
    }
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PlayerLookup(target_player) => {
            let stats = &prospect.stats[*target_player];
            stats.h1.goals > 0 || stats.h2.goals > 0
        }
        QuerySpec::NoAnytimeGoalscorer => !prospect
            .stats
            .iter()
            .any(|stats| stats.h1.goals > 0 || stats.h2.goals > 0),
        _ => panic!("{query:?} unsupported"),
    }
}