use crate::domain::{OfferType, OutcomeType, Over, Period, Player, Score, Side};
use racing_scraper::get_sports_contest;
use racing_scraper::sports::soccer::contest_model::ContestModel;
use racing_scraper::sports::soccer::market_model::{HomeAway, Scorer, SoccerMarket};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ContestSummary {
    pub id: String,
    pub name: String,
    pub offerings: HashMap<OfferType, HashMap<OutcomeType, f64>>,
}

impl From<ContestModel> for ContestSummary {
    fn from(external: ContestModel) -> Self {
        let id = external.id;
        let name = external.name;
        let mut offerings = HashMap::with_capacity(external.markets.len());
        for market in external.markets {
            match market {
                SoccerMarket::CorrectScore(markets) => {
                    offerings.insert(
                        OfferType::CorrectScore(Period::FullTime),
                        HashMap::from_iter(markets.iter().map(|market| {
                            (
                                OutcomeType::Score(Score {
                                    home: market.score.home as u8,
                                    away: market.score.away as u8,
                                }),
                                market.odds,
                            )
                        })),
                    );
                }
                SoccerMarket::TotalGoalsOverUnder(market, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::FullTime, Over(over)),
                        HashMap::from([
                            (OutcomeType::Over(over), market.over.unwrap_or(f64::INFINITY)),
                            (OutcomeType::Under(under), market.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::H2H(h2h) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::FullTime),
                        HashMap::from([
                            (OutcomeType::Win(Side::Home), h2h.home),
                            (OutcomeType::Win(Side::Away), h2h.away),
                            (OutcomeType::Draw, h2h.draw),
                        ]),
                    );
                }
                SoccerMarket::AnyTimeGoalScorer(scorers) => {
                    offerings.insert(
                        OfferType::AnytimeGoalscorer,
                        HashMap::from_iter(scorers.into_iter().map(|scorer| {
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(scorer);
                            (outcome_type, odds)
                        })),
                    );
                }
                SoccerMarket::FirstGoalScorer(scorers) => {
                    offerings.insert(
                        OfferType::FirstGoalscorer,
                        HashMap::from_iter(scorers.into_iter().map(|scorer| {
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(scorer);
                            (outcome_type, odds)
                        })),
                    );
                }
                SoccerMarket::CorrectScoreFirstHalf(markets) => {
                    offerings.insert(
                        OfferType::CorrectScore(Period::FirstHalf),
                        HashMap::from_iter(markets.iter().map(|market| {
                            (
                                OutcomeType::Score(Score {
                                    home: market.score.home as u8,
                                    away: market.score.away as u8,
                                }),
                                market.odds,
                            )
                        })),
                    );
                }
                SoccerMarket::CorrectScoreSecondHalf(markets) => {
                    offerings.insert(
                        OfferType::CorrectScore(Period::SecondHalf),
                        HashMap::from_iter(markets.iter().map(|market| {
                            (
                                OutcomeType::Score(Score {
                                    home: market.score.home as u8,
                                    away: market.score.away as u8,
                                }),
                                market.odds,
                            )
                        })),
                    );
                }
                SoccerMarket::TotalGoalsOddEven(_) => {
                    //TODO
                }
                SoccerMarket::FirstHalfGoalsOddEven(_) => {
                    //TODO
                }
                SoccerMarket::SecondHalfGoalOddEven(_) => {
                    //TODO
                }
                SoccerMarket::Score2GoalsOrMore(_) => {
                    //TODO
                }
                SoccerMarket::FirstHalfGoalsOverUnder(market, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::FirstHalf, Over(over)),
                        HashMap::from([
                            (OutcomeType::Over(over), market.over.unwrap_or(f64::INFINITY)),
                            (OutcomeType::Under(under), market.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::FirstHalfH2H(h2h) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::FirstHalf),
                        HashMap::from([
                            (OutcomeType::Win(Side::Home), h2h.home),
                            (OutcomeType::Win(Side::Away), h2h.away),
                            (OutcomeType::Draw, h2h.draw),
                        ]),
                    );
                }
                SoccerMarket::SecondHalfGoalsOverUnder(market, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::SecondHalf, Over(over)),
                        HashMap::from([
                            (OutcomeType::Over(over), market.over.unwrap_or(f64::INFINITY)),
                            (OutcomeType::Under(under), market.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::SecondHalfH2H(h2h) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::SecondHalf),
                        HashMap::from([
                            (OutcomeType::Win(Side::Home), h2h.home),
                            (OutcomeType::Win(Side::Away), h2h.away),
                            (OutcomeType::Draw, h2h.draw),
                        ]),
                    );
                }
            }
        }
        Self {
            id,
            name,
            offerings,
        }
    }
}

impl From<HomeAway> for Side {
    fn from(home_way: HomeAway) -> Self {
        match home_way {
            HomeAway::Home => Side::Home,
            HomeAway::Away => Side::Away,
        }
    }
}

struct OutcomeOdds(OutcomeType, f64);

impl From<Scorer> for OutcomeOdds {
    fn from(scorer: Scorer) -> Self {
        let outcome_type = match scorer.side {
            None => OutcomeType::None,
            Some(side) => OutcomeType::Player(Player::Named(side.into(), scorer.name)),
        };
        OutcomeOdds(outcome_type, scorer.odds)
    }
}

pub async fn download_by_id(id: String) -> anyhow::Result<ContestModel> {
    let contest = get_sports_contest(id).await?;
    Ok(contest)
}
