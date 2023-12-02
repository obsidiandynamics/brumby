use crate::domain::{OfferType, OutcomeType, Player};
use crate::interval::{Expansions, Prospect, Prospects};
use brumby::hash_lookup::HashLookup;

#[derive(Debug)]
pub enum QuerySpec {
    None,
    Generic(OfferType, OutcomeType),
    PlayerLookup(usize),
    NoFirstGoalscorer,
    NoAnytimeGoalscorer,
}

#[must_use]
pub fn requirements(offer_type: &OfferType) -> Expansions {
    match offer_type {
        OfferType::HeadToHead(period) => head_to_head::requirements(period),
        OfferType::TotalGoalsOverUnder(period, _) => total_goals::requirements(period),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::FirstGoalscorer => first_goalscorer::requirements(),
        OfferType::AnytimeGoalscorer => anytime_goalscorer::requirements(),
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[must_use]
pub fn prepare(
    offer_type: &OfferType,
    outcome_type: &OutcomeType,
    player_lookup: &HashLookup<Player>,
) -> QuerySpec {
    match offer_type {
        OfferType::HeadToHead(_) => head_to_head::prepare(offer_type, outcome_type),
        OfferType::TotalGoalsOverUnder(_, _) => total_goals::prepare(offer_type, outcome_type),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::FirstGoalscorer => first_goalscorer::prepare(outcome_type, player_lookup),
        OfferType::AnytimeGoalscorer => anytime_goalscorer::prepare(outcome_type, player_lookup),
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[must_use]
pub fn filter(offer_type: &OfferType, query: &QuerySpec, prospect: &Prospect) -> bool {
    match offer_type {
        OfferType::HeadToHead(_) => head_to_head::filter(query, prospect),
        OfferType::TotalGoalsOverUnder(_, _) => total_goals::filter(query, prospect),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::AnytimeGoalscorer => anytime_goalscorer::filter(query, prospect),
        OfferType::FirstGoalscorer => first_goalscorer::filter(query, prospect),
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[must_use]
pub fn isolate(
    offer_type: &OfferType,
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &HashLookup<Player>,
) -> f64 {
    let query = prepare(offer_type, outcome_type, player_lookup);
    prospects
        .iter()
        .filter(|(prospect, _)| filter(offer_type, &query, prospect))
        .map(|(_, prob)| prob)
        .sum()
    // prospects.iter().map(|(prospect, prob)| {
    //     if filter(offer_type, &query, prospect) {
    //         *prob
    //     } else {
    //         0.0
    //     }
    // }).sum()
}

#[must_use]
pub fn isolate_batch(
    selections: &[(OfferType, OutcomeType)],
    prospects: &Prospects,
    player_lookup: &HashLookup<Player>,
) -> f64 {
    let queries = selections
        .iter()
        .map(|(offer_type, outcome_type)| {
            (offer_type, prepare(offer_type, outcome_type, player_lookup))
        })
        .collect::<Vec<_>>();
    let mut prob_sum = 0.0;
    for (prospect, prospect_prob) in prospects {
        let mut all_pass = true;
        for (offer_type, query) in queries.iter() {
            if !filter(offer_type, query, prospect) {
                all_pass = false;
                break;
            }
        }
        if all_pass {
            prob_sum += prospect_prob;
        }
    }
    prob_sum
}

mod head_to_head {
    use super::*;
    use crate::domain::{Period, Side};

    #[inline]
    #[must_use]
    pub(crate) fn requirements(period: &Period) -> Expansions {
        match period {
            Period::FirstHalf | Period::SecondHalf => Expansions {
                ft_score: false,
                player_stats: true,
                player_split_stats: false,
                first_goalscorer: false,
            },
            Period::FullTime => Expansions {
                ft_score: true,
                player_stats: false,
                player_split_stats: false,
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
                    Period::FirstHalf => todo!(),
                    Period::SecondHalf => todo!(),
                    Period::FullTime => (prospect.score.home, prospect.score.away),
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
}

mod total_goals {
    use crate::domain::Period;
    use super::*;

    #[inline]
    #[must_use]
    pub(crate) fn requirements(period: &Period) -> Expansions {
        match period {
            Period::FirstHalf | Period::SecondHalf => Expansions {
                ft_score: false,
                player_stats: true,
                player_split_stats: false,
                first_goalscorer: false,
            },
            Period::FullTime => Expansions {
                ft_score: true,
                player_stats: false,
                player_split_stats: false,
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
            QuerySpec::Generic(OfferType::TotalGoalsOverUnder(period, _), outcome_type) => {
                let (home_goals, away_goals) = match period {
                    Period::FirstHalf => todo!(),
                    Period::SecondHalf => todo!(),
                    Period::FullTime => (prospect.score.home, prospect.score.away),
                };

                match outcome_type {
                    OutcomeType::Over(limit) => home_goals + away_goals > *limit,
                    OutcomeType::Under(limit) => away_goals + home_goals < *limit,
                    _ => panic!("{outcome_type:?} unsupported"),
                }
            }
            _ => panic!("{query:?} unsupported"),
        }
    }

}

mod first_goalscorer {
    use super::*;

    #[inline]
    #[must_use]
    pub(crate) fn requirements() -> Expansions {
        Expansions {
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
}

mod anytime_goalscorer {
    use super::*;

    #[inline]
    #[must_use]
    pub(crate) fn requirements() -> Expansions {
        Expansions {
            ft_score: false,
            player_stats: true,
            player_split_stats: false,
            first_goalscorer: false,
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
            OutcomeType::None => QuerySpec::NoAnytimeGoalscorer,
            _ => panic!("{outcome_type:?} unsupported"),
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
}
