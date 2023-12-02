use brumby::lookup::Lookup;
use crate::domain::{OfferType, OutcomeType, Player};
use crate::interval::{Expansions, Prospect, Prospects};

#[derive(Debug)]
pub enum QuerySpec {
    None,
    PlayerLookup(usize),
    NoFirstGoalscorer,
    NoAnytimeGoalscorer
}

#[must_use]
pub fn requirements(offer_type: &OfferType) -> Expansions {
    match offer_type {
        OfferType::HeadToHead(_) => unimplemented!(),
        OfferType::TotalGoalsOverUnder(_, _) => unimplemented!(),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::FirstGoalscorer => {
            requirements_first_goalscorer()
        }
        OfferType::AnytimeGoalscorer => {
            requirements_anytime_goalscorer()
        }
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[inline]
#[must_use]
fn requirements_first_goalscorer() -> Expansions {
    Expansions {
        ft_score: false,
        player_stats: false,
        player_split_stats: false,
        first_goalscorer: true,
    }
}

#[inline]
#[must_use]
fn requirements_anytime_goalscorer() -> Expansions {
    Expansions {
        ft_score: false,
        player_stats: true,
        player_split_stats: false,
        first_goalscorer: false,
    }
}

#[must_use]
pub fn prepare(
    offer_type: &OfferType,
    outcome_type: &OutcomeType,
    player_lookup: &Lookup<Player>,
) -> QuerySpec {
    match offer_type {
        OfferType::HeadToHead(_) => unimplemented!(),
        OfferType::TotalGoalsOverUnder(_, _) => unimplemented!(),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::FirstGoalscorer => {
            prepare_first_goalscorer(outcome_type, player_lookup)
        }
        OfferType::AnytimeGoalscorer => {
            prepare_anytime_goalscorer(outcome_type, player_lookup)
        }
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[inline]
#[must_use]
fn prepare_first_goalscorer(
    outcome_type: &OutcomeType,
    player_lookup: &Lookup<Player>,
) -> QuerySpec {
    match outcome_type {
        OutcomeType::Player(player) => {
            QuerySpec::PlayerLookup(player_lookup.index_of(player).unwrap())
        },
        OutcomeType::None => QuerySpec::NoFirstGoalscorer,
        _ => panic!("{outcome_type:?} unsupported"),
    }
}

#[inline]
#[must_use]
fn prepare_anytime_goalscorer(
    outcome_type: &OutcomeType,
    player_lookup: &Lookup<Player>,
) -> QuerySpec {
    match outcome_type {
        OutcomeType::Player(player) => {
            QuerySpec::PlayerLookup(player_lookup.index_of(player).unwrap())
        },
        OutcomeType::None => QuerySpec::NoAnytimeGoalscorer,
        _ => panic!("{outcome_type:?} unsupported"),
    }
}

#[must_use]
pub fn filter(offer_type: &OfferType,
              query: &QuerySpec,
              prospect: &Prospect) -> bool {
    match offer_type {
        OfferType::HeadToHead(_) => unimplemented!(),
        OfferType::TotalGoalsOverUnder(_, _) => unimplemented!(),
        OfferType::CorrectScore(_) => unimplemented!(),
        OfferType::DrawNoBet => unimplemented!(),
        OfferType::AnytimeGoalscorer => {
            filter_anytime_goalscorer(query, prospect)
        }
        OfferType::FirstGoalscorer => {
            filter_first_goalscorer(query, prospect)
        }
        OfferType::PlayerShotsOnTarget(_) => unimplemented!(),
        OfferType::AnytimeAssist => unimplemented!(),
    }
}

#[inline]
#[must_use]
fn filter_first_goalscorer(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PlayerLookup(target_player) => match prospect.first_scorer {
            None => false,
            Some(scorer) => scorer == *target_player,
        },
        QuerySpec::NoFirstGoalscorer => prospect.first_scorer.is_none(),
        _ => panic!("{query:?} unsupported"),
    }
}

#[inline]
#[must_use]
fn filter_anytime_goalscorer(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PlayerLookup(target_player) => {
            let stats = &prospect.stats[*target_player];
            stats.h1.goals > 0 || stats.h2.goals > 0
        },
        QuerySpec::NoAnytimeGoalscorer => {
            !prospect.stats.iter().any(|stats| stats.h1.goals > 0 || stats.h2.goals > 0)
        }
        _ => panic!("{query:?} unsupported"),
    }
}

#[must_use]
pub fn isolate(
    offer_type: &OfferType,
    outcome_type: &OutcomeType,
    prospects: &Prospects,
    player_lookup: &Lookup<Player>,
) -> f64 {
    let query = prepare(offer_type, outcome_type, player_lookup);
    prospects.iter().filter(|(prospect, _)| filter(offer_type, &query, prospect)).map(|(_, prob)|prob).sum()
    // prospects.iter().map(|(prospect, prob)| {
    //     if filter(offer_type, &query, prospect) {
    //         *prob
    //     } else {
    //         0.0
    //     }
    // }).sum()
}