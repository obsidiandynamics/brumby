use super::*;

#[inline]
#[must_use]
pub(crate) fn requirements() -> Expansions {
    Expansions {
        ht_score: false,
        ft_score: false,
        player_stats: false,
        player_split_stats: false,
        first_goalscorer: true,
    }
}

#[inline]
#[must_use]
pub(crate) fn prepare(
    outcome_type: &OutcomeType,
    player_lookup: &HashLookup<Player>,
) -> QuerySpec {
    match outcome_type {
        OutcomeType::Player(player) => {
            QuerySpec::PlayerLookup(player_lookup.index_of(player).unwrap())
        }
        OutcomeType::None => QuerySpec::NoFirstGoalscorer,
        _ => panic!("{outcome_type:?} unsupported"),
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