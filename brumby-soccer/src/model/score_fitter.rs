use std::ops::RangeInclusive;
use anyhow::bail;
use crate::domain::{Offer, OfferCategory, OfferType, OutcomeType, Over, Period};
use crate::model::{FitError, MissingOffer, Model};
use rustc_hash::FxHashMap;
use tracing::debug;
use crate::fit;

pub struct Config {
    h1_goal_ratio: f64,
    // poisson_search: HypergridSearchConfig<'a>,
    // binomial_search: HypergridSearchConfig<'a>,
}
impl Config {
    fn validate(&self) -> Result<(), anyhow::Error> {
        const H1_GOAL_RATIO_RANGE: RangeInclusive<f64> = 0.0..=1.0;
        if !H1_GOAL_RATIO_RANGE.contains(&self.h1_goal_ratio) {
            bail!("H1 goal ratio ({}) outside of allowable range (H1_GOAL_RATIO_RANGE:?)", self.h1_goal_ratio);
        }
        // self.poisson_search.validate()?;
        // self.binomial_search.validate()?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            h1_goal_ratio: 0.425
        }
    }
}

pub struct ScoreFitter {
    config: Config,
}
impl ScoreFitter {
    pub fn fit(&self, model: &mut Model, offers: &FxHashMap<OfferType, Offer>) -> Result<(), FitError> {
        let (ft_goals, _) =
            most_balanced_goals(offers.values(), &Period::FullTime).ok_or_else(|| {
                FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals))
            })?;

        let ft_h2h = get_offer(offers, &OfferType::HeadToHead(Period::FullTime))?;
        let (ft_search_outcome, lambdas) = fit::fit_scoregrid_full(&ft_h2h, &ft_goals);

        let (h1_goals, h1_goals_over) =
            most_balanced_goals(offers.values(), &Period::FirstHalf).ok_or_else(|| {
                FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals))
            })?;
        let h1_h2h = get_offer(offers, &OfferType::HeadToHead(Period::FirstHalf))?;

        let (h2_goals, h2_goals_over) =
            most_balanced_goals(offers.values(), &Period::SecondHalf).ok_or_else(|| {
                FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals))
            })?;
        let h2_h2h = get_offer(offers, &OfferType::HeadToHead(Period::SecondHalf))?;

        debug!("fitting 1st half ({:.1} goals line)", h1_goals_over.0 as f64 + 0.5);
        let h1_home_goals_estimate = (lambdas[0] + lambdas[2]) * self.config.h1_goal_ratio;
        let h1_away_goals_estimate = (lambdas[1] + lambdas[2]) * self.config.h1_goal_ratio;
        let h1_search_outcome = fit::fit_scoregrid_half(h1_home_goals_estimate, h1_away_goals_estimate, &[&h1_h2h, &h1_goals]);

        debug!("fitting 2nd half ({:.1} goals line)", h2_goals_over.0 as f64 + 0.5);
        let h2_home_goals_estimate = (lambdas[0] + lambdas[2]) * (1.0 - self.config.h1_goal_ratio);
        let h2_away_goals_estimate = (lambdas[1] + lambdas[2]) * (1.0 - self.config.h1_goal_ratio);
        let h2_search_outcome = fit::fit_scoregrid_half(h2_home_goals_estimate, h2_away_goals_estimate, &[&h2_h2h, &h2_goals]);

        Ok(())
    }
}

fn get_offer<'a>(offers: &'a FxHashMap<OfferType, Offer>, offer_type: &OfferType) -> Result<&'a Offer, MissingOffer> {
    offers.get(offer_type).ok_or_else(|| MissingOffer::Type(offer_type.clone()))
}

impl TryFrom<Config> for ScoreFitter {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self { config })
    }
}

fn most_balanced_goals<'a>(
    offers: impl Iterator<Item = &'a Offer>,
    period: &Period,
) -> Option<(&'a Offer, &'a Over)> {
    let mut most_balanced = None;
    let mut most_balanced_diff = f64::MAX;
    for offer in offers {
        match &offer.offer_type {
            OfferType::TotalGoals(p, over) => {
                if p == period {
                    let diff = f64::abs(offer.market.prices[0] - offer.market.prices[1]);
                    if diff < most_balanced_diff {
                        most_balanced_diff = diff;
                        most_balanced = Some((offer, over));
                    }
                }
            }
            _ => {}
        }
    }

    most_balanced
}
