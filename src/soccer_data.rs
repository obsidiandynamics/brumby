use crate::entity::{MarketType, OutcomeType, Over, Player, Score, Side};
use racing_scraper::get_sports_contest;
use racing_scraper::sports::soccer::contest_model::ContestModel;
use racing_scraper::sports::soccer::market_model::{HomeAway, Scorer, SoccerMarket};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ContestSummary {
    pub id: String,
    pub name: String,
    pub offerings: HashMap<MarketType, HashMap<OutcomeType, f64>>,
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
                        MarketType::CorrectScore,
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
                        MarketType::TotalGoalsOverUnder(Over(over)),
                        HashMap::from([
                            (OutcomeType::Over(over), market.over),
                            (OutcomeType::Under(under), market.under),
                        ]),
                    );
                }
                SoccerMarket::H2H(h2h) => {
                    offerings.insert(
                        MarketType::HeadToHead,
                        HashMap::from([
                            (OutcomeType::Win(Side::Home), h2h.home),
                            (OutcomeType::Win(Side::Away), h2h.away),
                            (OutcomeType::Draw, h2h.draw),
                        ]),
                    );
                }
                SoccerMarket::AnyTimeGoalScorer(scorers) => {
                    offerings.insert(
                        MarketType::AnytimeGoalscorer,
                        HashMap::from_iter(scorers.into_iter().map(|scorer| {
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(scorer);
                            (outcome_type, odds)
                        })),
                    );
                }
                SoccerMarket::FirstGoalScorer(scorers) => {
                    offerings.insert(
                        MarketType::FirstGoalscorer,
                        HashMap::from_iter(scorers.into_iter().map(|scorer| {
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(scorer);
                            (outcome_type, odds)
                        })),
                    );
                }
                SoccerMarket::CorrectScoreFirstHalf(_) => {
                    //TODO
                }
                SoccerMarket::CorrectScoreSecondHalf(_) => {
                    //TODO
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
