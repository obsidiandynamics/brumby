use super::*;
use crate::domain::{Period, Side};

#[inline]
#[must_use]
pub(crate) fn requirements(period: &Period) -> Expansions {
    match period {
        Period::FirstHalf => Expansions {
            ht_score: true,
            ft_score: false,
            player_goal_stats: false,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
        Period::SecondHalf => Expansions {
            ht_score: true,
            ft_score: true,
            player_goal_stats: false,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
        Period::FullTime => Expansions {
            ht_score: false,
            ft_score: true,
            player_goal_stats: false,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
    }
}

#[inline]
#[must_use]
pub(crate) fn prepare(offer_type: &OfferType, outcome_type: &OutcomeType) -> QuerySpec {
    QuerySpec::Generic(offer_type.clone(), outcome_type.clone())
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::Generic(OfferType::HeadToHead(period), outcome_type) => {
            let (home_goals, away_goals) = match period {
                Period::FirstHalf => (prospect.ht_score.home, prospect.ht_score.away),
                Period::SecondHalf => { let h2_score = prospect.h2_score(); (h2_score.home, h2_score.away) },
                Period::FullTime => (prospect.ft_score.home, prospect.ft_score.away),
            };

            match outcome_type {
                OutcomeType::Win(Side::Home) => home_goals > away_goals,
                OutcomeType::Win(Side::Away) => away_goals > home_goals,
                OutcomeType::Draw => home_goals == away_goals,
                _ => panic!("{outcome_type:?} unsupported"),
            }
        }
        _ => panic!("{query:?} unsupported"),
    }
}