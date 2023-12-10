use std::ops::Range;

use rustc_hash::FxHashMap;

use brumby::hash_lookup::HashLookup;

use crate::domain::{Player, Score, Side};

pub mod assist;
pub mod query;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Prospect {
    pub ht_score: Score,
    pub ft_score: Score,
    pub stats: Vec<PlayerStats>,
    pub first_scorer: Option<usize>,
}
impl Prospect {
    fn init(players: usize) -> Prospect {
        let stats = vec![PlayerStats::default(); players];
        Prospect {
            ht_score: Score::nil_all(),
            ft_score: Score::nil_all(),
            stats,
            first_scorer: None,
        }
    }

    fn h2_score(&self) -> Score {
        Score {
            home: self.ft_score.home - self.ht_score.home,
            away: self.ft_score.away - self.ht_score.away,
        }
    }
}

pub type Prospects = FxHashMap<Prospect, f64>;

pub fn init_prospects(capacity: usize) -> Prospects {
    Prospects::with_capacity_and_hasher(capacity, Default::default())
}

#[derive(Debug, Clone)]
pub struct UnivariateProbs {
    pub home: f64,
    pub away: f64,
}

impl<'a> From<&'a [f64]> for UnivariateProbs {
    fn from(params: &'a [f64]) -> Self {
        assert_eq!(2, params.len());
        Self {
            home: params[0],
            away: params[1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BivariateProbs {
    pub home: f64,
    pub away: f64,
    pub common: f64,
}

impl<'a> From<&'a [f64]> for BivariateProbs {
    fn from(params: &'a [f64]) -> Self {
        assert_eq!(3, params.len());
        Self {
            home: params[0],
            away: params[1],
            common: params[2],
        }
    }
}

#[derive(Debug)]
pub struct Expansions {
    pub ht_score: bool,
    pub ft_score: bool,
    pub player_goal_stats: bool,
    pub player_split_goal_stats: bool,
    pub max_player_assists: u8,
    pub first_goalscorer: bool,
}
impl Expansions {
    fn validate(&self) {
        if self.player_split_goal_stats {
            assert!(
                self.player_goal_stats,
                "cannot expand player split goal stats without player goal stats"
            );
        }
        assert!(
            self.ft_score
                || self.ht_score
                || self.player_goal_stats
                || self.first_goalscorer
                || self.max_player_assists > 0,
            "at least one expansion must be enabled"
        )
    }
}

impl Default for Expansions {
    fn default() -> Self {
        Self {
            ft_score: true,
            ht_score: true,
            player_goal_stats: true,
            player_split_goal_stats: true,
            max_player_assists: u8::MAX,
            first_goalscorer: true,
        }
    }
}

#[derive(Debug)]
pub struct PruneThresholds {
    pub max_total_goals: u16,
    pub min_prob: f64,
}
impl Default for PruneThresholds {
    fn default() -> Self {
        Self {
            max_total_goals: u16::MAX,
            min_prob: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct TeamProbs {
    pub h1_goals: BivariateProbs,
    pub h2_goals: BivariateProbs,
    pub assists: UnivariateProbs,
}

#[derive(Debug)]
pub struct IntervalConfig {
    pub intervals: u8,
    pub team_probs: TeamProbs,
    pub player_probs: Vec<(Player, PlayerProbs)>,
    pub prune_thresholds: PruneThresholds,
    pub expansions: Expansions,
}

#[derive(Debug)]
pub struct PlayerProbs {
    pub goal: Option<f64>,
    pub assist: Option<f64>,
}

#[derive(Debug)]
pub struct Exploration {
    pub player_lookup: HashLookup<Player>,
    pub prospects: Prospects,
    pub pruned: f64,
}

#[derive(Debug)]
struct PartialProspect<'a> {
    home_scorer: Option<usize>,
    away_scorer: Option<usize>,
    home_assister: Option<usize>,
    away_assister: Option<usize>,
    first_scoring_side: Option<&'a Side>,
    prob: f64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PeriodStats {
    pub goals: u8,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PlayerStats {
    pub h1: PeriodStats,
    pub h2: PeriodStats,
    pub assists: u8,
}

#[derive(Debug, Eq, PartialEq)]
enum Half {
    First,
    Second,
}

pub fn explore(config: &IntervalConfig, include_intervals: Range<u8>) -> Exploration {
    config.expansions.validate();

    let mut player_lookup = HashLookup::with_capacity(config.player_probs.len() + 1);
    let mut home_scorers = Vec::with_capacity(config.player_probs.len() + 1);
    let mut away_scorers = Vec::with_capacity(config.player_probs.len() + 1);
    let mut home_assisters = Vec::with_capacity(config.player_probs.len() + 1);
    let mut away_assisters = Vec::with_capacity(config.player_probs.len() + 1);
    let mut combined_home_player_goal_prob = 0.0;
    let mut combined_away_player_goal_prob = 0.0;
    for (player_index, (player, player_probs)) in config.player_probs.iter().enumerate() {
        player_lookup.push(player.clone());
        match player {
            Player::Named(side, _) => match side {
                Side::Home => {
                    if let Some(prob) = player_probs.goal {
                        combined_home_player_goal_prob += prob;
                        home_scorers.push((player_index, prob));
                    }
                    if let Some(prob) = player_probs.assist {
                        home_assisters.push((player_index, prob));
                    }
                }
                Side::Away => {
                    if let Some(prob) = player_probs.goal {
                        combined_away_player_goal_prob += prob;
                        away_scorers.push((player_index, prob));
                    }
                    if let Some(prob) = player_probs.assist {
                        away_assisters.push((player_index, prob));
                    }
                }
            },
            Player::Other => panic!("unsupported scorer {:?}", Player::Other),
        }
    }
    player_lookup.push(Player::Other);
    // let other_player_index = config.player_probs.len();
    home_scorers.push((
        config.player_probs.len(),
        1.0 - combined_home_player_goal_prob,
    ));
    away_scorers.push((
        config.player_probs.len(),
        1.0 - combined_away_player_goal_prob,
    ));
    home_assisters.push((config.player_probs.len(), f64::NAN)); // the probability for 'other' is derived on the fly
    away_assisters.push((config.player_probs.len(), f64::NAN));

    let mut current_prospects = init_prospects(1);
    current_prospects.insert(Prospect::init(player_lookup.len()), 1.0);
    let mut pruned = 0.0;

    for interval in include_intervals {
        let half = if interval < config.intervals / 2 {
            Half::First
        } else {
            Half::Second
        };
        let params = match half {
            Half::First => &config.team_probs.h1_goals,
            Half::Second => &config.team_probs.h2_goals,
        };

        let neither_prob = 1.0 - params.home - params.away - params.common;
        let mut next_prospects = init_prospects((current_prospects.len() as f64 * 1.1) as usize);

        for (current_prospect, current_prob) in current_prospects {
            if current_prob < config.prune_thresholds.min_prob {
                pruned += current_prob;
                continue;
            }

            // neither team scores
            let partial = PartialProspect {
                home_scorer: None,
                away_scorer: None,
                home_assister: None,
                away_assister: None,
                first_scoring_side: None,
                prob: neither_prob,
            };
            merge(
                &config.expansions,
                &half,
                &current_prospect,
                current_prob,
                partial,
                &mut next_prospects,
            );

            // at least one more goal allowed before pruning
            if current_prospect.ft_score.total() < config.prune_thresholds.max_total_goals {
                // only the home team scores
                for (scorer_index, player_score_prob) in &home_scorers {
                    for (assister, player_assist_prob) in assist::Iter::new(
                        config.team_probs.assists.home,
                        &home_assisters,
                        *scorer_index,
                    ) {
                        merge(
                            &config.expansions,
                            &half,
                            &current_prospect,
                            current_prob,
                            PartialProspect {
                                home_scorer: Some(*scorer_index),
                                away_scorer: None,
                                home_assister: assister,
                                away_assister: None,
                                first_scoring_side: Some(&Side::Home),
                                prob: params.home * player_score_prob * player_assist_prob,
                            },
                            &mut next_prospects,
                        );
                    }
                    // let mut remaining_player_assist_prob = 1.0;
                    // for (assister_index, player_assist_prob) in &home_assisters {
                    //     if *scorer_index != other_player_index && assister_index == scorer_index {
                    //         continue;
                    //     }
                    //     let player_assist_prob = if assister_index == &other_player_index {
                    //         remaining_player_assist_prob
                    //     } else {
                    //         remaining_player_assist_prob -= player_assist_prob;
                    //         *player_assist_prob
                    //     };
                    //
                    //     for (assister, assist_prob) in [
                    //         (Some(*assister_index), config.team_probs.assists.home * player_assist_prob),
                    //         (None, (1.0 - config.team_probs.assists.home) * player_assist_prob)
                    //     ] {
                    //         if assist_prob == 0.0 {
                    //             continue;
                    //         }
                    //         merge(
                    //             &config.expansions,
                    //             &half,
                    //             &current_prospect,
                    //             current_prob,
                    //             PartialProspect {
                    //                 home_scorer: Some(*scorer_index),
                    //                 away_scorer: None,
                    //                 home_assister: assister,
                    //                 away_assister: None,
                    //                 first_scoring_side: Some(&Side::Home),
                    //                 prob: params.home
                    //                     * player_score_prob
                    //                     * assist_prob,
                    //             },
                    //             &mut next_prospects,
                    //         );
                    //     }
                    // }
                }

                // only the away team scores
                for (scorer_index, player_score_prob) in &away_scorers {
                    for (assister, player_assist_prob) in assist::Iter::new(
                        config.team_probs.assists.away,
                        &away_assisters,
                        *scorer_index,
                    ) {
                        merge(
                            &config.expansions,
                            &half,
                            &current_prospect,
                            current_prob,
                            PartialProspect {
                                home_scorer: None,
                                away_scorer: Some(*scorer_index),
                                home_assister: None,
                                away_assister: assister,
                                first_scoring_side: Some(&Side::Away),
                                prob: params.away * player_score_prob * player_assist_prob,
                            },
                            &mut next_prospects,
                        );
                    }
                    // let mut remaining_player_assist_prob = 1.0;
                    // for (assister_index, player_assist_prob) in &away_assisters {
                    //     if *scorer_index != other_player_index && assister_index == scorer_index {
                    //         continue;
                    //     }
                    //     let player_assist_prob = if assister_index == &other_player_index {
                    //         remaining_player_assist_prob
                    //     } else {
                    //         remaining_player_assist_prob -= player_assist_prob;
                    //         *player_assist_prob
                    //     };
                    //
                    //     for (assister, assist_prob) in [
                    //         (
                    //             Some(*assister_index),
                    //             config.team_probs.assists.away * player_assist_prob,
                    //         ),
                    //         (
                    //             None,
                    //             (1.0 - config.team_probs.assists.away) * player_assist_prob,
                    //         ),
                    //     ] {
                    //         if assist_prob == 0.0 {
                    //             continue;
                    //         }
                    //         merge(
                    //             &config.expansions,
                    //             &half,
                    //             &current_prospect,
                    //             current_prob,
                    //             PartialProspect {
                    //                 home_scorer: None,
                    //                 away_scorer: Some(*scorer_index),
                    //                 home_assister: None,
                    //                 away_assister: assister,
                    //                 first_scoring_side: Some(&Side::Away),
                    //                 prob: params.away * player_score_prob * assist_prob,
                    //             },
                    //             &mut next_prospects,
                    //         );
                    //     }
                    // }
                }
            } else {
                pruned += current_prob * (params.home + params.away);
            }

            // at least two more goals allowed before pruning
            if current_prospect.ft_score.total() + 1 < config.prune_thresholds.max_total_goals {
                // both teams score
                for (home_scorer_index, home_player_score_prob) in &home_scorers {
                    for (away_scorer_index, away_player_score_prob) in &away_scorers {
                        for first_scoring_side in [&Side::Home, &Side::Away] {
                            for (home_assister, home_player_assist_prob) in assist::Iter::new(
                                config.team_probs.assists.home,
                                &home_assisters,
                                *home_scorer_index,
                            ) {
                                for (away_assister, away_player_assist_prob) in assist::Iter::new(
                                    config.team_probs.assists.away,
                                    &away_assisters,
                                    *away_scorer_index,
                                ) {
                                    merge(
                                        &config.expansions,
                                        &half,
                                        &current_prospect,
                                        current_prob,
                                        PartialProspect {
                                            home_scorer: Some(*home_scorer_index),
                                            away_scorer: Some(*away_scorer_index),
                                            home_assister,
                                            away_assister,
                                            first_scoring_side: Some(first_scoring_side),
                                            prob: params.common
                                                * 0.5
                                                * home_player_score_prob
                                                * away_player_score_prob
                                                * home_player_assist_prob
                                                * away_player_assist_prob,
                                        },
                                        &mut next_prospects,
                                    );
                                }
                            }
                            // let mut remaining_home_player_assist_prob = 1.0;
                            // for (home_assister_index, home_player_assist_prob) in &home_assisters {
                            //     if *home_scorer_index != other_player_index
                            //         && home_assister_index == home_scorer_index
                            //     {
                            //         continue;
                            //     }
                            //     let home_player_assist_prob = if home_assister_index
                            //         == &other_player_index
                            //     {
                            //         remaining_home_player_assist_prob
                            //     } else {
                            //         remaining_home_player_assist_prob -= home_player_assist_prob;
                            //         *home_player_assist_prob
                            //     };
                            //
                            //     // with assist
                            //     let mut remaining_away_player_assist_prob = 1.0;
                            //     for (away_assister_index, away_player_assist_prob) in
                            //         &away_assisters
                            //     {
                            //         if *away_scorer_index != other_player_index
                            //             && away_assister_index == away_scorer_index
                            //         {
                            //             continue;
                            //         }
                            //         let away_player_assist_prob =
                            //             if away_assister_index == &other_player_index {
                            //                 remaining_away_player_assist_prob
                            //             } else {
                            //                 remaining_away_player_assist_prob -=
                            //                     away_player_assist_prob;
                            //                 *away_player_assist_prob
                            //             };
                            //
                            //         for (home_assister, away_assister, assist_prob) in [
                            //             (
                            //                 Some(*home_assister_index),
                            //                 Some(*away_assister_index),
                            //                 config.team_probs.assists.home
                            //                     * home_player_assist_prob
                            //                     * config.team_probs.assists.away
                            //                     * away_player_assist_prob,
                            //             ),
                            //             (
                            //                 Some(*home_assister_index),
                            //                 None,
                            //                 config.team_probs.assists.home
                            //                     * home_player_assist_prob
                            //                     * (1.0 - config.team_probs.assists.away)
                            //                     * away_player_assist_prob,
                            //             ),
                            //             (
                            //                 None,
                            //                 Some(*away_assister_index),
                            //                 (1.0 - config.team_probs.assists.home)
                            //                     * home_player_assist_prob
                            //                     * config.team_probs.assists.away
                            //                     * away_player_assist_prob,
                            //             ),
                            //             (
                            //                 None,
                            //                 None,
                            //                 (1.0 - config.team_probs.assists.home)
                            //                     * home_player_assist_prob
                            //                     * (1.0 - config.team_probs.assists.away)
                            //                     * away_player_assist_prob,
                            //             ),
                            //         ] {
                            //             if assist_prob == 0.0 {
                            //                 continue;
                            //             }
                            //
                            //             merge(
                            //                 &config.expansions,
                            //                 &half,
                            //                 &current_prospect,
                            //                 current_prob,
                            //                 PartialProspect {
                            //                     home_scorer: Some(*home_scorer_index),
                            //                     away_scorer: Some(*away_scorer_index),
                            //                     home_assister,
                            //                     away_assister,
                            //                     first_scoring_side: Some(first_scoring_side),
                            //                     prob: params.common
                            //                         * 0.5
                            //                         * home_player_score_prob
                            //                         * away_player_score_prob
                            //                         * assist_prob,
                            //                 },
                            //                 &mut next_prospects,
                            //             );
                            //         }
                            //     }
                            // }
                        }
                    }
                }
            } else {
                pruned += current_prob * params.common;
            }
        }

        current_prospects = next_prospects;
    }

    Exploration {
        player_lookup,
        prospects: current_prospects,
        pruned,
    }
}

#[inline]
fn merge(
    expansions: &Expansions,
    half: &Half,
    current_prospect: &Prospect,
    current_prob: f64,
    partial: PartialProspect,
    next_prospects: &mut Prospects,
) {
    let merged_prob = current_prob * partial.prob;
    let mut merged = current_prospect.clone();
    if let Some(player) = partial.home_scorer {
        if expansions.player_split_goal_stats {
            let split_stats = match half {
                Half::First => &mut merged.stats[player].h1,
                Half::Second => &mut merged.stats[player].h2,
            };
            split_stats.goals += 1;
        } else if expansions.player_goal_stats {
            merged.stats[player].h2.goals += 1;
        }

        if expansions.ft_score {
            merged.ft_score.home += 1;
        }
        if expansions.ht_score && half == &Half::First {
            merged.ht_score.home += 1;
        }

        if expansions.first_goalscorer
            && merged.first_scorer.is_none()
            && partial.first_scoring_side.unwrap() == &Side::Home
        {
            merged.first_scorer = Some(player);
        }
    }
    if let Some(player) = partial.away_scorer {
        if expansions.player_split_goal_stats {
            let split_stats = match half {
                Half::First => &mut merged.stats[player].h1,
                Half::Second => &mut merged.stats[player].h2,
            };
            split_stats.goals += 1;
        } else if expansions.player_goal_stats {
            merged.stats[player].h2.goals += 1;
        }

        if expansions.ft_score {
            merged.ft_score.away += 1;
        }
        if expansions.ht_score && half == &Half::First {
            merged.ht_score.away += 1;
        }

        if expansions.first_goalscorer
            && merged.first_scorer.is_none()
            && partial.first_scoring_side.unwrap() == &Side::Away
        {
            merged.first_scorer = Some(player);
        }
    }

    if let Some(player) = partial.home_assister {
        if merged.stats[player].assists < expansions.max_player_assists {
            merged.stats[player].assists += 1;
        }
    }
    if let Some(player) = partial.away_assister {
        if merged.stats[player].assists < expansions.max_player_assists {
            merged.stats[player].assists += 1;
        }
    }

    next_prospects
        .entry(merged)
        .and_modify(|prob| *prob += merged_prob)
        .or_insert(merged_prob);
}

#[cfg(test)]
mod tests;
