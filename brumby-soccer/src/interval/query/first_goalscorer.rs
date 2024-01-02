use super::*;

#[inline]
#[must_use]
pub(crate) fn requirements() -> Expansions {
    Expansions {
        ht_score: false,
        ft_score: false,
        max_player_goals: 0,
        player_split_goal_stats: false,
        max_player_assists: 0,
        first_goalscorer: true,
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
        Outcome::None => QuerySpec::NoFirstGoalscorer,
        _ => panic!("{outcome:?} unsupported"),
    }
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PlayerLookup(target_player) => match prospect.first_scorer {
            None => false,
            Some(scorer) => scorer == *target_player,
        },
        QuerySpec::NoFirstGoalscorer => prospect.first_scorer.is_none(),
        _ => panic!("{query:?} unsupported"),
    }
}