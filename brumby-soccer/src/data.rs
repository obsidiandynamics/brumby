use crate::domain::{DrawHandicap, OfferType, Outcome, Over, Period, Player, Score, Side, WinHandicap};
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
    pub offerings: FxHashMap<OfferType, HashMap<Outcome, f64>>,
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
                                Outcome::Score(Score {
                                    home: market.score.home as u8,
                                    away: market.score.away as u8,
                                }),
                                market.odds,
                            )
                        })),
                    );
                }
                SoccerMarket::TotalGoalsOverUnder(prices, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::FullTime, Over(over)),
                        HashMap::from([
                            (Outcome::Over(over), prices.over.unwrap_or(f64::INFINITY)),
                            (Outcome::Under(under), prices.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::H2H(prices) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
                        HashMap::from([
                            (Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), prices.home),
                            (Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)), prices.away),
                            (Outcome::Draw(DrawHandicap::Ahead(0)), prices.draw),
                        ]),
                    );
                }
                SoccerMarket::AnytimeGoalScorer(players) => {
                    offerings.insert(
                        OfferType::AnytimeGoalscorer,
                        HashMap::from_iter(players.into_iter().map(|player| {
                            let OutcomeOdds(outcome, odds) = OutcomeOdds::from(player);
                            (outcome, odds)
                        })),
                    );
                }
                SoccerMarket::FirstGoalScorer(players) => {
                    offerings.insert(
                        OfferType::FirstGoalscorer,
                        HashMap::from_iter(players.into_iter().map(|player| {
                            let OutcomeOdds(outcome, odds) = OutcomeOdds::from(player);
                            (outcome, odds)
                        })),
                    );
                }
                SoccerMarket::CorrectScoreFirstHalf(markets) => {
                    offerings.insert(
                        OfferType::CorrectScore(Period::FirstHalf),
                        HashMap::from_iter(markets.iter().map(|market| {
                            (
                                Outcome::Score(Score {
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
                                Outcome::Score(Score {
                                    home: market.score.home as u8,
                                    away: market.score.away as u8,
                                }),
                                market.odds,
                            )
                        })),
                    );
                }
                SoccerMarket::Score2GoalsOrMore(_) => {
                    //TODO
                }
                SoccerMarket::FirstHalfGoalsOverUnder(prices, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::FirstHalf, Over(over)),
                        HashMap::from([
                            (Outcome::Over(over), prices.over.unwrap_or(f64::INFINITY)),
                            (Outcome::Under(under), prices.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::FirstHalfH2H(prices) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::FirstHalf, DrawHandicap::Ahead(0)),
                        HashMap::from([
                            (Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), prices.home),
                            (Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)), prices.away),
                            (Outcome::Draw(DrawHandicap::Ahead(0)), prices.draw),
                        ]),
                    );
                }
                SoccerMarket::SecondHalfGoalsOverUnder(prices, line) => {
                    let (over, under) = (line.floor() as u8, line.ceil() as u8);
                    offerings.insert(
                        OfferType::TotalGoals(Period::SecondHalf, Over(over)),
                        HashMap::from([
                            (Outcome::Over(over), prices.over.unwrap_or(f64::INFINITY)),
                            (Outcome::Under(under), prices.under.unwrap_or(f64::INFINITY)),
                        ]),
                    );
                }
                SoccerMarket::SecondHalfH2H(prices) => {
                    offerings.insert(
                        OfferType::HeadToHead(Period::SecondHalf, DrawHandicap::Ahead(0)),
                        HashMap::from([
                            (Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), prices.home),
                            (Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)), prices.away),
                            (Outcome::Draw(DrawHandicap::Ahead(0)), prices.draw),
                        ]),
                    );
                }
                SoccerMarket::PlayerAssist(players, at_least) => {
                    if at_least == 1 {
                        offerings.insert(
                            OfferType::AnytimeAssist,
                            HashMap::from_iter(players.into_iter().map(|player| {
                                let OutcomeOdds(outcome, odds) = OutcomeOdds::from(player);
                                (outcome, odds)
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
                SoccerMarket::TwoWayHandicap(prices, handicap) => {
                    let win_handicap = to_win_handicap(handicap);
                    // println!("two-way handicap: {handicap}, prices: {prices:?}");
                    // println!("home: {:?}, away: {:?}", win_handicap, win_handicap.flip_asian());
                    offerings.insert(
                        OfferType::AsianHandicap(Period::FullTime, win_handicap.clone()),
                        HashMap::from([
                            (Outcome::Win(Side::Home, win_handicap.clone()), prices.home),
                            (Outcome::Win(Side::Away, win_handicap.flip_asian()), prices.away),
                        ]),
                    );
                }
                SoccerMarket::FirstHalfTwoWayHandicap(prices, handicap) => {
                    let win_handicap = to_win_handicap(handicap);
                    offerings.insert(
                        OfferType::AsianHandicap(Period::FirstHalf, win_handicap.clone()),
                        HashMap::from([
                            (Outcome::Win(Side::Home, win_handicap.clone()), prices.home),
                            (Outcome::Win(Side::Away, win_handicap.flip_asian()), prices.away),
                        ]),
                    );
                }
                SoccerMarket::SecondHalfTwoWayHandicap(prices, handicap) => {
                    let win_handicap = to_win_handicap(handicap);
                    offerings.insert(
                        OfferType::AsianHandicap(Period::SecondHalf, win_handicap.clone()),
                        HashMap::from([
                            (Outcome::Win(Side::Home, win_handicap.clone()), prices.home),
                            (Outcome::Win(Side::Away, win_handicap.flip_asian()), prices.away),
                        ]),
                    );
                }
                SoccerMarket::ThreeWayHandicap(prices, handicap) => {
                    // println!("three-way handicap: {handicap}, prices: {h2h:?}");
                    let draw_handicap = to_draw_handicap(handicap);
                    let win_handicap = draw_handicap.to_win_handicap();
                    offerings.insert(
                        OfferType::HeadToHead(Period::FullTime, draw_handicap.clone()),
                        HashMap::from([
                            (Outcome::Win(Side::Home, win_handicap.clone()), prices.home),
                            (Outcome::Win(Side::Away, win_handicap.flip_european()), prices.away),
                            (Outcome::Draw(draw_handicap), prices.draw),
                        ]),
                    );
                }
                SoccerMarket::DrawNoBet(prices) => {
                    let draw_handicap = DrawHandicap::Ahead(0);
                    offerings.insert(
                        OfferType::DrawNoBet(draw_handicap.clone()),
                        HashMap::from([
                            (Outcome::Win(Side::Home, draw_handicap.to_win_handicap()), prices.home),
                            (Outcome::Win(Side::Away, draw_handicap.to_win_handicap().flip_european()), prices.away),
                        ])
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

fn to_draw_handicap(handicap: i8) -> DrawHandicap {
    if handicap > 0 {
        DrawHandicap::Behind(handicap as u8)
    } else {
        DrawHandicap::Ahead(-handicap as u8)
    }
}

fn to_win_handicap(handicap: f32) -> WinHandicap {
    if handicap > 0.0 {
        WinHandicap::BehindUnder(handicap as u8 + 1)
    } else {
        WinHandicap::AheadOver(-handicap as u8)
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

struct OutcomeOdds(Outcome, f64);

impl From<ScraperPlayer> for OutcomeOdds {
    fn from(player: ScraperPlayer) -> Self {
        let outcome = match player.side {
            None => Outcome::None,
            Some(side) => Outcome::Player(Player::Named(side.into(), player.name)),
        };
        OutcomeOdds(outcome, player.odds)
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
