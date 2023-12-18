use rustc_hash::FxHashMap;
use crate::domain::{Offer, OfferType, OutcomeType, Over, Period};
use crate::model::{FitError, Model};

pub struct FitterConfig {
    // poisson_search: HypergridSearchConfig<'a>,
    // binomial_search: HypergridSearchConfig<'a>,
}
impl FitterConfig {
    fn validate(&self) -> Result<(), anyhow::Error> {
        // self.poisson_search.validate()?;
        // self.binomial_search.validate()?;
        Ok(())
    }
}

pub struct ScoreFitter {
    config: FitterConfig
}
impl ScoreFitter {
    pub fn fit(model: &mut Model, offers: &FxHashMap<OutcomeType, Offer>) -> Result<(), FitError> {
        let ft_goals = most_balanced_total_goals(offers.values(), &Period::FullTime);
        todo!()
    }
}

impl TryFrom<FitterConfig> for ScoreFitter {
    type Error = anyhow::Error;

    fn try_from(config: FitterConfig) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self {
            config,
        })
    }
}

fn most_balanced_total_goals<'a>(offers: impl Iterator<Item = &'a Offer>, period: &Period) -> Option<(&'a Offer, &'a Over)> {
    let mut most_balanced = None;
    let mut most_balanced_diff = f64::MAX;
    for offer in offers {
        match &offer.offer_type {
            OfferType::TotalGoals(p, over) => if p == period {
                let diff = offer.market.prices[0] - offer.market.prices[1];
                if diff < most_balanced_diff {
                    most_balanced_diff = diff;
                    most_balanced = Some((offer, over));
                }
            }
            _ => {}
        }
    }

    most_balanced
}