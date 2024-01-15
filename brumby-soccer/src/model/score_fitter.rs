use std::ops::RangeInclusive;

use anyhow::anyhow;
use rustc_hash::FxHashMap;
use tracing::debug;

use brumby::probs::Fraction;

use crate::domain::{DrawHandicap, Offer, OfferCategory, OfferType, Period};
use crate::domain::validation::OfferCapture;
use crate::fit;
use crate::interval::BivariateProbs;
use crate::model::{
    FitError, get_offer, GoalProbs, MissingOffer, Model, most_balanced_goals, ValidationError,
};

pub struct Config {
    h1_goal_ratio: f64,
    half_total_goals_ratio: Fraction,
}
impl Config {
    fn validate(&self) -> Result<(), ValidationError> {
        const H1_GOAL_RATIO_RANGE: RangeInclusive<f64> = 0.0..=1.0;
        if !H1_GOAL_RATIO_RANGE.contains(&self.h1_goal_ratio) {
            return Err(anyhow!(
                "H1 goal ratio ({}) outside of allowable range (H1_GOAL_RATIO_RANGE:?)",
                self.h1_goal_ratio
            )
            .into());
        }
        if self.half_total_goals_ratio.numerator >= self.half_total_goals_ratio.denominator {
            return Err(anyhow!("half total goals ratio is not a proper fraction").into());
        }
        if self.half_total_goals_ratio.numerator == 0 {
            return Err(anyhow!("half total goals ratio cannot be zero").into());
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            h1_goal_ratio: 0.425,
            half_total_goals_ratio: Fraction {
                numerator: 3,
                denominator: 4,
            },
        }
    }
}

pub struct ScoreFitter {
    config: Config,
}
impl ScoreFitter {
    pub fn fit(
        &self,
        model: &mut Model,
        offers: &FxHashMap<OfferType, Offer>,
    ) -> Result<(), FitError> {
        let (ft_goals, _) =
            most_balanced_goals(offers.values(), &Period::FullTime).ok_or(
                FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals))
            )?;
        let ft_goals = OfferCapture::try_from(ft_goals)?;

        let ft_h2h =
            OfferCapture::try_from(get_offer(offers, &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)))?)?;
        let (ft_search_outcome, lambdas) = fit::fit_scoregrid_full(
            &ft_h2h,
            &ft_goals,
            model.config.intervals,
            model.config.max_total_goals,
        );

        let (h1_goals, h1_goals_over) = most_balanced_goals(offers.values(), &Period::FirstHalf)
            .ok_or(
                FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals))
            )?;
        let h1_goals = OfferCapture::try_from(h1_goals)?;
        let h1_h2h = OfferCapture::try_from(get_offer(
            offers,
            &OfferType::HeadToHead(Period::FirstHalf, DrawHandicap::Ahead(0)),
        )?)?;

        let (h2_goals, h2_goals_over) = most_balanced_goals(offers.values(), &Period::SecondHalf)
            .ok_or(
            FitError::MissingOffer(MissingOffer::Category(OfferCategory::TotalGoals)))?;
        let h2_goals = OfferCapture::try_from(h2_goals)?;
        let h2_h2h = OfferCapture::try_from(get_offer(
            offers,
            &OfferType::HeadToHead(Period::SecondHalf, DrawHandicap::Ahead(0)),
        )?)?;

        debug!(
            "fitting 1st half ({:.1} goals line)",
            h1_goals_over.0 as f64 + 0.5
        );
        let h1_home_goals_estimate = (lambdas[0] + lambdas[2]) * self.config.h1_goal_ratio;
        let h1_away_goals_estimate = (lambdas[1] + lambdas[2]) * self.config.h1_goal_ratio;
        let max_total_goals_half =
            (model.config.max_total_goals as u64 * self.config.half_total_goals_ratio.numerator
                / self.config.half_total_goals_ratio.denominator) as u16;
        let h1_search_outcome = fit::fit_scoregrid_half(
            h1_home_goals_estimate,
            h1_away_goals_estimate,
            &[&h1_h2h, &h1_goals],
            model.config.intervals,
            max_total_goals_half,
        );

        debug!(
            "fitting 2nd half ({:.1} goals line)",
            h2_goals_over.0 as f64 + 0.5
        );
        let h2_home_goals_estimate = (lambdas[0] + lambdas[2]) * (1.0 - self.config.h1_goal_ratio);
        let h2_away_goals_estimate = (lambdas[1] + lambdas[2]) * (1.0 - self.config.h1_goal_ratio);
        let h2_search_outcome = fit::fit_scoregrid_half(
            h2_home_goals_estimate,
            h2_away_goals_estimate,
            &[&h2_h2h, &h2_goals],
            model.config.intervals,
            max_total_goals_half,
        );

        let (mut adj_optimal_h1, mut adj_optimal_h2) = ([0.0; 3], [0.0; 3]);
        // only adjust the home and away scoring probs; common prob is locked to the full-time one
        for (i, &orig_h1) in h1_search_outcome.optimal_values.iter().enumerate() {
            let orig_h2 = h2_search_outcome.optimal_values[i];
            let ft = ft_search_outcome.optimal_values[i];
            let avg_h1_h2 = (orig_h1 + orig_h2) / 2.0;
            if avg_h1_h2 > 0.0 {
                adj_optimal_h1[i] = orig_h1 / (avg_h1_h2 / ft);
                adj_optimal_h2[i] = orig_h2 / (avg_h1_h2 / ft);
            } else {
                adj_optimal_h1[i] = orig_h1;
                adj_optimal_h2[i] = orig_h2;
            }
        }
        adj_optimal_h1[2] = ft_search_outcome.optimal_values[2];
        adj_optimal_h2[2] = ft_search_outcome.optimal_values[2];
        model.goal_probs = Some(GoalProbs {
            h1: BivariateProbs::from(&adj_optimal_h1),
            h2: BivariateProbs::from(&adj_optimal_h2),
        });

        Ok(())
    }
}

impl TryFrom<Config> for ScoreFitter {
    type Error = ValidationError;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        config.validate()?;
        Ok(Self { config })
    }
}
