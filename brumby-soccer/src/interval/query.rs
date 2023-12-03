use crate::domain::{OfferType, OutcomeType, Player};
use crate::interval::{Expansions, Prospect, Prospects};
use brumby::hash_lookup::HashLookup;

mod correct_score;
mod head_to_head;
mod total_goals;
mod first_goalscorer;
mod anytime_goalscorer;

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
        OfferType::TotalGoals(period, _) => total_goals::requirements(period),
        OfferType::CorrectScore(period) => correct_score::requirements(period),
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
        OfferType::TotalGoals(_, _) => total_goals::prepare(offer_type, outcome_type),
        OfferType::CorrectScore(_) => correct_score::prepare(offer_type, outcome_type),
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
        OfferType::TotalGoals(_, _) => total_goals::filter(query, prospect),
        OfferType::CorrectScore(_) => correct_score::filter(query, prospect),
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
    prospects
        .iter()
        .filter(|(prospect, _)| {
            queries
                .iter()
                .any(|(offer_type, query)| !filter(offer_type, query, prospect))
        })
        .map(|(_, prospect_prob)| prospect_prob)
        .sum()
}
