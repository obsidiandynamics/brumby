use super::*;
use crate::domain::Period;

#[inline]
#[must_use]
pub(crate) fn requirements(period: &Period) -> Expansions {
    match period {
        Period::FirstHalf => Expansions {
            ht_score: true,
            ft_score: false,
            max_player_goals: 0,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
        Period::SecondHalf => Expansions {
            ht_score: true,
            ft_score: true,
            max_player_goals: 0,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
        Period::FullTime => Expansions {
            ht_score: false,
            ft_score: true,
            max_player_goals: 0,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        },
    }
}

#[inline]
#[must_use]
pub(crate) fn prepare(offer_type: &OfferType, outcome: &Outcome) -> QuerySpec {
    QuerySpec::Generic(offer_type.clone(), outcome.clone())
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::Generic(OfferType::TotalGoals(period, _), outcome) => {
            let (home_goals, away_goals) = match period {
                Period::FirstHalf => (prospect.ht_score.home, prospect.ht_score.away),
                Period::SecondHalf => { let h2_score = prospect.h2_score(); (h2_score.home, h2_score.away) },
                Period::FullTime => (prospect.ft_score.home, prospect.ft_score.away),
            };

            match outcome {
                Outcome::Over(limit) => home_goals + away_goals > *limit,
                Outcome::Under(limit) => away_goals + home_goals < *limit,
                _ => panic!("{outcome:?} unsupported"),
            }
        }
        _ => panic!("{query:?} unsupported"),
    }
}