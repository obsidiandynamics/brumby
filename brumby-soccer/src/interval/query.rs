use brumby::hash_lookup::HashLookup;

use crate::domain::{OfferType, OutcomeType, Player};
use crate::interval::{Expansions, Prospect, Prospects};

mod anytime_assist;
mod anytime_goalscorer;
mod correct_score;
mod first_goalscorer;
mod head_to_head;
mod total_goals;

#[derive(Debug)]
pub enum QuerySpec {
    None,
    Generic(OfferType, OutcomeType),
    PlayerLookup(usize),
    NoFirstGoalscorer,
    NoAnytimeGoalscorer,
    NoAnytimeAssist,
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
        OfferType::AnytimeAssist => anytime_assist::requirements(),
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
        OfferType::AnytimeAssist => anytime_assist::prepare(outcome_type, player_lookup),
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
        OfferType::AnytimeAssist => anytime_assist::filter(query, prospect),
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
pub fn isolate_set(
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
            !queries
                .iter()
                .any(|(offer_type, query)| !filter(offer_type, query, prospect))
        })
        .map(|(_, prospect_prob)| prospect_prob)
        .sum()
}

#[cfg(test)]
mod tests {
    use brumby::sv;
    use crate::domain::{Period, Score, Side};
    use crate::interval::{explore, Config, BivariateProbs, TeamProbs, UnivariateProbs};

    use super::*;

    #[test]
    fn isolate_degenerate_case_of_isolate_set() {
        let exploration = explore(
            &Config {
                intervals: 4,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    assists: UnivariateProbs { home: 1.0, away: 1.0 },
                },
                player_probs: sv![],
                prune_thresholds: Default::default(),
                expansions: Expansions {
                    ht_score: false,
                    ft_score: true,
                    max_player_goals: u8::MAX,
                    player_split_goal_stats: false,
                    max_player_assists: 0,
                    first_goalscorer: false,
                },
            },
            0..4,
        );
        let home_win = isolate(
            &OfferType::HeadToHead(Period::FullTime),
            &OutcomeType::Win(Side::Home),
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert!(home_win > 0.0, "{home_win}");

        let home_win_set = isolate_set(
            &[(
                OfferType::HeadToHead(Period::FullTime),
                OutcomeType::Win(Side::Home),
            )],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!(home_win, home_win_set);
    }

    #[test]
    fn logical_implication_is_a_subset() {
        let exploration = explore(
            &Config {
                intervals: 4,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    assists: UnivariateProbs { home: 1.0, away: 1.0 },
                },
                player_probs: sv![],
                prune_thresholds: Default::default(),
                expansions: Expansions {
                    ht_score: false,
                    ft_score: true,
                    max_player_goals: u8::MAX,
                    player_split_goal_stats: false,
                    max_player_assists: 0,
                    first_goalscorer: false,
                },
            },
            0..4,
        );

        let home_win = isolate_set(
            &[(
                OfferType::HeadToHead(Period::FullTime),
                OutcomeType::Win(Side::Home),
            )],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert!(home_win > 0.0, "{home_win}");

        let one_nil = isolate_set(
            &[(
                OfferType::CorrectScore(Period::FullTime),
                OutcomeType::Score(Score { home: 1, away: 0 }),
            )],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert!(one_nil > 0.0, "{one_nil}");
        assert!(home_win > one_nil);

        let one_nil_and_home_win = isolate_set(
            &[
                (
                    OfferType::CorrectScore(Period::FullTime),
                    OutcomeType::Score(Score { home: 1, away: 0 }),
                ),
                (
                    OfferType::HeadToHead(Period::FullTime),
                    OutcomeType::Win(Side::Home),
                ),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!(one_nil, one_nil_and_home_win);
    }

    #[test]
    fn impossibility_of_conflicting_outcomes() {
        let exploration = explore(
            &Config {
                intervals: 4,
                team_probs: TeamProbs {
                    h1_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    h2_goals: BivariateProbs { home: 0.25, away: 0.25, common: 0.25 },
                    assists: UnivariateProbs { home: 1.0, away: 1.0 },
                },
                player_probs: sv![],
                prune_thresholds: Default::default(),
                expansions: Expansions {
                    ht_score: false,
                    ft_score: true,
                    max_player_goals: u8::MAX,
                    player_split_goal_stats: false,
                    max_player_assists: 0,
                    first_goalscorer: false,
                },
            },
            0..4,
        );

        let home_win = isolate_set(
            &[(
                OfferType::HeadToHead(Period::FullTime),
                OutcomeType::Win(Side::Home),
            )],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert!(home_win > 0.0, "{home_win}");

        let nil_one = isolate_set(
            &[(
                OfferType::CorrectScore(Period::FullTime),
                OutcomeType::Score(Score { home: 0, away: 1 }),
            )],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert!(nil_one > 0.0, "{nil_one}");

        let nil_one_home_win = isolate_set(
            &[
                (
                    OfferType::CorrectScore(Period::FullTime),
                    OutcomeType::Score(Score { home: 0, away: 1 }),
                ),
                (
                    OfferType::HeadToHead(Period::FullTime),
                    OutcomeType::Win(Side::Home),
                ),
            ],
            &exploration.prospects,
            &exploration.player_lookup,
        );
        assert_eq!(0.0, nil_one_home_win);
    }
}
