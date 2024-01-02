use super::*;
use crate::domain::{DrawHandicap, Period, Side, WinHandicap};

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
    QuerySpec::PassThrough(offer_type.clone(), outcome.clone())
}

#[inline]
#[must_use]
pub(crate) fn filter(query: &QuerySpec, prospect: &Prospect) -> bool {
    match query {
        QuerySpec::PassThrough(OfferType::HeadToHead(period, _), outcome) => {
            let (home_goals, away_goals) = match period {
                Period::FirstHalf => (prospect.ht_score.home, prospect.ht_score.away),
                Period::SecondHalf => {
                    let h2_score = prospect.h2_score();
                    (h2_score.home, h2_score.away)
                }
                Period::FullTime => (prospect.ft_score.home, prospect.ft_score.away),
            };

            match outcome {
                Outcome::Win(Side::Home, win_handicap) => match win_handicap {
                    WinHandicap::AheadOver(by) => home_goals.saturating_sub(away_goals) > *by,
                    WinHandicap::BehindUnder(by) => {
                        home_goals > away_goals || away_goals - home_goals < *by
                    }
                },
                Outcome::Win(Side::Away, win_handicap) => match win_handicap {
                    WinHandicap::AheadOver(by) => away_goals.saturating_sub(home_goals) > *by,
                    WinHandicap::BehindUnder(by) => {
                        away_goals > home_goals || home_goals - away_goals < *by
                    }
                },
                Outcome::Draw(draw_handicap) => match draw_handicap {
                    DrawHandicap::Ahead(by) => {
                        home_goals >= away_goals && home_goals - away_goals == *by
                    }
                    DrawHandicap::Behind(by) => {
                        away_goals >= home_goals && away_goals - home_goals == *by
                    }
                },
                _ => panic!("{outcome:?} unsupported"),
            }
        }
        _ => panic!("{query:?} unsupported"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Score;
    use crate::interval::Exploration;
    use assert_float_eq::*;
    use brumby::linear::matrix::Matrix;

    fn create_test_4x4_exploration() -> Exploration {
        let mut scoregrid = Matrix::allocate(4, 4);
        scoregrid[0].copy_from_slice(&[0.04, 0.03, 0.02, 0.01]);
        scoregrid[1].copy_from_slice(&[0.08, 0.06, 0.04, 0.02]);
        scoregrid[2].copy_from_slice(&[0.12, 0.09, 0.06, 0.03]);
        scoregrid[3].copy_from_slice(&[0.16, 0.12, 0.08, 0.04]);

        let mut prospects = Prospects::default();
        for home_goals in 0..scoregrid.rows() {
            for away_goals in 0..scoregrid.cols() {
                let prob = scoregrid[(home_goals, away_goals)];
                prospects.insert(
                    Prospect {
                        ht_score: Score::nil_all(),
                        ft_score: Score {
                            home: home_goals as u8,
                            away: away_goals as u8,
                        },
                        stats: Default::default(),
                        first_scorer: None,
                    },
                    prob,
                );
            }
        }
        Exploration {
            player_lookup: HashLookup::default(),
            prospects,
            pruned: 0.0,
        }
    }

    #[test]
    pub fn win_gather() {
        let exploration = create_test_4x4_exploration();
        assert_float_absolute_eq!(
            0.65,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
                &Outcome::Win(Side::Home, WinHandicap::AheadOver(0)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
        assert_float_absolute_eq!(
            0.15,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
                &Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
    }

    #[test]
    pub fn win_handicap_1_gather() {
        let exploration = create_test_4x4_exploration();
        assert_float_absolute_eq!(
            0.4,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(1)),
                &Outcome::Win(Side::Home, WinHandicap::AheadOver(1)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
        assert_float_absolute_eq!(
            0.35,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(1)),
                &Outcome::Win(Side::Away, WinHandicap::BehindUnder(1)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
    }

    #[test]
    pub fn draw_gather() {
        let exploration = create_test_4x4_exploration();
        assert_float_absolute_eq!(
            0.2,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
                &Outcome::Draw(DrawHandicap::Ahead(0)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
    }

    #[test]
    pub fn draw_handicap_1_gather() {
        let exploration = create_test_4x4_exploration();
        assert_float_absolute_eq!(
            0.25,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(1)),
                &Outcome::Draw(DrawHandicap::Ahead(1)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
        assert_float_absolute_eq!(
            0.1,
            isolate(
                &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(1)),
                &Outcome::Draw(DrawHandicap::Behind(1)),
                &exploration.prospects,
                &exploration.player_lookup
            )
        );
    }
}
