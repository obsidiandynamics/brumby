use std::ops::{AddAssign, Range};

use bincode::Encode;
use rustc_hash::FxHashMap;

use brumby::hash_lookup::HashLookup;
use brumby::stack_vec::StackVec;
use brumby::sv;

use crate::domain::{Player, Score, Side};

mod assist;
pub mod query;

pub const NUM_PLAYERS: usize = 3;
pub const NUM_PLAYER_STATS: usize = NUM_PLAYERS + 1;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Prospect {
    pub ht_score: Score,
    pub ft_score: Score,
    pub stats: StackVec<PlayerStats, NUM_PLAYER_STATS>,
    pub first_scorer: Option<usize>,
}
impl Prospect {
    fn init(players: usize) -> Prospect {
        let stats = sv![PlayerStats::default(); players];
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

    fn total_goals(&self) -> u16 {
        u16::max(self.ht_score.total(), self.ft_score.total())
    }
}

pub type Prospects = FxHashMap<Prospect, f64>;

pub fn init_prospects(capacity: usize) -> Prospects {
    Prospects::with_capacity_and_hasher(capacity, Default::default())
}

#[derive(Debug, Clone, Default, Encode)]
pub struct UnivariateProbs {
    pub home: f64,
    pub away: f64,
}

impl<'a> From<&'a [f64; 2]> for UnivariateProbs {
    fn from(params: &'a [f64; 2]) -> Self {
        Self {
            home: params[0],
            away: params[1],
        }
    }
}

#[derive(Debug, Clone, Default, Encode)]
pub struct BivariateProbs {
    pub home: f64,
    pub away: f64,
    pub common: f64,
}

impl<'a> From<&'a [f64; 3]> for BivariateProbs {
    fn from(params: &'a [f64; 3]) -> Self {
        Self {
            home: params[0],
            away: params[1],
            common: params[2],
        }
    }
}

#[derive(Debug, Clone, Encode)]
pub struct Expansions {
    pub ht_score: bool,
    pub ft_score: bool,
    pub max_player_goals: u8,
    pub player_split_goal_stats: bool,
    pub max_player_assists: u8,
    pub first_goalscorer: bool,
}
impl Expansions {
    fn validate(&self) {
        if self.player_split_goal_stats {
            assert!(
                self.max_player_goals > 0,
                "cannot expand player split goal stats without player goals"
            );
        }
        assert!(
            self.ft_score
                || self.ht_score
                || self.max_player_goals > 0
                || self.first_goalscorer
                || self.max_player_assists > 0,
            "at least one expansion must be enabled"
        )
    }

    pub fn requires_team_goal_probs(&self) -> bool {
        self.ht_score || self.ft_score || self.max_player_goals > 0 || self.first_goalscorer || self.max_player_assists > 0
    }

    pub fn requires_team_assist_probs(&self) -> bool {
        self.max_player_assists > 0
    }

    pub fn requires_player_goal_probs(&self) -> bool {
        self.max_player_goals > 0 || self.first_goalscorer
    }

    pub fn requires_player_assist_probs(&self) -> bool {
        self.max_player_assists > 0
    }
    
    pub fn empty() -> Self {
        Self {
            ht_score: false,
            ft_score: false,
            max_player_goals: 0,
            player_split_goal_stats: false,
            max_player_assists: 0,
            first_goalscorer: false,
        }
    }
}

impl Default for Expansions {
    fn default() -> Self {
        Self {
            ft_score: true,
            ht_score: true,
            max_player_goals: u8::MAX,
            player_split_goal_stats: true,
            max_player_assists: u8::MAX,
            first_goalscorer: true,
        }
    }
}

// impl Add for Expansions {
//     type Output = Expansions;
//
//     fn add(self, rhs: Self) -> Self::Output {
//         Self {
//             ht_score: self.ht_score || rhs.ht_score,
//             ft_score: self.ft_score || rhs.ft_score,
//             max_player_goals: u8::max(self.max_player_goals, rhs.max_player_goals),
//             player_split_goal_stats: self.player_split_goal_stats || rhs.player_split_goal_stats,
//             max_player_assists: u8::max(self.max_player_assists, rhs.max_player_assists),
//             first_goalscorer: self.first_goalscorer || rhs.first_goalscorer,
//         }
//     }
// }

impl AddAssign for Expansions {
    fn add_assign(&mut self, rhs: Self) {
        self.ht_score |= rhs.ht_score;
        self.ft_score |= rhs.ft_score;
        self.max_player_goals = u8::max(self.max_player_goals, rhs.max_player_goals);
        self.player_split_goal_stats |= rhs.player_split_goal_stats;
        self.max_player_assists = u8::max(self.max_player_assists, rhs.max_player_assists);
        self.first_goalscorer |= rhs.first_goalscorer;
    }
}

#[derive(Debug, Clone, Encode)]
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

#[derive(Debug, Clone, Encode)]
pub struct TeamProbs {
    pub h1_goals: BivariateProbs,
    pub h2_goals: BivariateProbs,
    pub assists: UnivariateProbs,
}

#[derive(Debug, Encode)]
pub struct Config {
    pub intervals: u8,
    pub team_probs: TeamProbs,
    pub player_probs: StackVec<(Player, PlayerProbs), NUM_PLAYERS>,
    pub prune_thresholds: PruneThresholds,
    pub expansions: Expansions,
}

#[derive(Debug, Default, Encode, Clone)]
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct PeriodStats {
    pub goals: u8,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
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

pub fn explore(config: &Config, include_intervals: Range<u8>) -> Exploration {
    config.expansions.validate();

    let mut player_lookup = HashLookup::with_capacity(config.player_probs.len() + 1);
    let mut home_scorers = StackVec::<_, NUM_PLAYER_STATS>::default();
    let mut away_scorers = StackVec::<_, NUM_PLAYER_STATS>::default();
    let mut home_assisters = StackVec::<_, NUM_PLAYER_STATS>::default();
    let mut away_assisters = StackVec::<_, NUM_PLAYER_STATS>::default();
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
            if current_prospect.total_goals() < config.prune_thresholds.max_total_goals {
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
                }
            } else {
                pruned += current_prob * (params.home + params.away);
            }

            // at least two more goals allowed before pruning
            if current_prospect.total_goals() + 1 < config.prune_thresholds.max_total_goals {
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
            if split_stats.goals < expansions.max_player_goals {
                split_stats.goals += 1;
            }
        } else if merged.stats[player].h2.goals < expansions.max_player_goals {
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
            if split_stats.goals < expansions.max_player_goals {
                split_stats.goals += 1;
            }
        } else if merged.stats[player].h2.goals < expansions.max_player_goals {
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
