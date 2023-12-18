use crate::domain::{OfferType, OutcomeType, Over, Period, Player, Score, Side};
use racing_scraper::sports::soccer::contest_model::ContestModel;
use racing_scraper::sports::soccer::market_model::{HomeAway, Player as ScraperPlayer, SoccerMarket};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use racing_scraper::sports::{get_sports_contest, Provider};
use rustc_hash::FxHashMap;
use thiserror::Error;
use brumby::feed_id::FeedId;

#[derive(Debug)]
pub struct ContestSummary {
    pub id: String,
    pub name: String,
    pub offerings: FxHashMap<OfferType, HashMap<OutcomeType, f64>>,
}

impl From<ContestModel> for ContestSummary {
    fn from(external: ContestModel) -> Self {
        let id = external.id;
        let name = external.name;
        let mut offerings = FxHashMap::with_capacity_and_hasher(external.markets.len(), Default::default());
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
                SoccerMarket::AnytimeGoalScorer(players) => {
                    offerings.insert(
                        OfferType::AnytimeGoalscorer,
                        HashMap::from_iter(players.into_iter().map(|player| {
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(player);
                            (outcome_type, odds)
                        })),
                    );
                }
                SoccerMarket::FirstGoalScorer(players) => {
                    offerings.insert(
                        OfferType::FirstGoalscorer,
                        HashMap::from_iter(players.into_iter().map(|player| {
                            // if player.side.is_none() {
                            //     println!("PLAYER {player:?}");
                            // }
                            let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(player);
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
                // SoccerMarket::TotalGoalsOddEven(_) => {
                //     //TODO
                // }
                // SoccerMarket::FirstHalfGoalsOddEven(_) => {
                //     //TODO
                // }
                // SoccerMarket::SecondHalfGoalOddEven(_) => {
                //     //TODO
                // }
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
                SoccerMarket::PlayerAssist(players, at_least) => {
                    if at_least == 1 {
                        offerings.insert(
                            OfferType::AnytimeAssist,
                            HashMap::from_iter(players.into_iter().map(|player| {
                                let OutcomeOdds(outcome_type, odds) = OutcomeOdds::from(player);
                                (outcome_type, odds)
                            })),
                        );
                    }
                }
                SoccerMarket::TotalCardsOverUnder(_, _) => {}
                SoccerMarket::FirstHalfCardsOverUnder(_, _) => {}
                SoccerMarket::SecondHalfCardsOverUnder(_, _) => {}
                SoccerMarket::PlayerShotsWoodwork(_, _) => {}
                SoccerMarket::PlayerTotalShots(_, _) => {}
                SoccerMarket::PlayerShotsOnTarget(_, _) => {}
                SoccerMarket::TotalCornersOverUnder(_, _) => {}
                SoccerMarket::PlayerShownCard(_) => {}
                SoccerMarket::PlayerShotsOutsideBox(_, _) => {}
                SoccerMarket::FirstHalfCornersOverUnder(_, _) => {}
                SoccerMarket::SecondHalfCornersOverUnder(_, _) => {}
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

impl From<ScraperPlayer> for OutcomeOdds {
    fn from(player: ScraperPlayer) -> Self {
        let outcome_type = match player.side {
            None => OutcomeType::None,
            Some(side) => OutcomeType::Player(Player::Named(side.into(), player.name)),
        };
        OutcomeOdds(outcome_type, player.odds)
    }
}

#[derive(Debug, Clone)]
pub struct DataProvider(pub Provider);

impl From<DataProvider> for Provider {
    fn from(value: DataProvider) -> Self {
        value.0
    }
}

impl FromStr for DataProvider {
    type Err = ProviderParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ladbrokes" => Ok(DataProvider(Provider::Ladbrokes)),
            "pointsbet" => Ok(DataProvider(Provider::PointsBet)),
            _ => Err(ProviderParseError(format!("unsupported provider {s}")))
        }
    }
}

#[derive(Error, Debug)]
pub struct ProviderParseError(String);

impl Display for ProviderParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type SoccerFeedId = FeedId<DataProvider>;

pub async fn download_by_id(id: SoccerFeedId) -> anyhow::Result<ContestModel> {
    let (provider, entity_id) = id.take();
    let contest = get_sports_contest(provider.into(), entity_id).await?;
    Ok(contest)
}
