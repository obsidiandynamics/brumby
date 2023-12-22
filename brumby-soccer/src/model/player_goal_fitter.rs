use rustc_hash::FxHashMap;
use tracing::debug;

use crate::domain::{Offer, OfferType, OutcomeType};
use crate::domain::error::{MissingOutcome, OfferCapture};
use crate::fit;
use crate::model::{FitError, get_offer, Model};

pub struct PlayerGoalFitter;
impl PlayerGoalFitter {
    pub fn fit(
        &self,
        model: &mut Model,
        offers: &FxHashMap<OfferType, Offer>,
    ) -> Result<(), FitError> {
        let goal_probs = model.require_team_goal_probs()?;
        let first_goalscorer =
            OfferCapture::try_from(get_offer(offers, &OfferType::FirstGoalscorer)?)?;
        let nil_all_draw_prob =
            first_goalscorer
                .get_probability(&OutcomeType::None)
                .ok_or(MissingOutcome {
                    offer_type: OfferType::FirstGoalscorer,
                    outcome_type: OutcomeType::None,
                })?;
        debug!("nil-all draw prob: {nil_all_draw_prob}");

        let fitted_goalscorer_probs = fit::fit_first_goalscorer_all(
            &goal_probs.h1,
            &goal_probs.h2,
            &first_goalscorer,
            nil_all_draw_prob,
            model.config.intervals,
            model.config.max_total_goals,
        );

        let anytime_goalscorer =
            OfferCapture::try_from(get_offer(offers, &OfferType::AnytimeGoalscorer)?)?;
        let prob_est_adj =
            first_goalscorer.market.fair_booksum() / anytime_goalscorer.market.fair_booksum();

        let mut fitted_goalscorer_probs = fitted_goalscorer_probs.into_iter().collect::<FxHashMap<_, _>>();

        // truncated anytime goalscorer offer that only contains player outcomes lacking a goal probability
        let anytime_goalscorer =
            anytime_goalscorer.subset(|(outcome, _)| match outcome.get_player() {
                None => false,
                Some(player) => !fitted_goalscorer_probs.contains_key(player)
            });

        if let Some(anytime_goalscorer) = anytime_goalscorer {
            debug!("fitting anytime goalscorer for extras {:?}", anytime_goalscorer.outcomes.items());
            let extra_fitted_goalscorer_probs = fit::fit_anytime_goalscorer_all(
                &goal_probs.h1,
                &goal_probs.h2,
                &anytime_goalscorer,
                nil_all_draw_prob,
                prob_est_adj,
                model.config.intervals,
                model.config.max_total_goals,
            );
            for (player, player_goal_prob) in extra_fitted_goalscorer_probs {
                fitted_goalscorer_probs.insert(player, player_goal_prob);
            }
        }

        for (player, player_goal_prob) in fitted_goalscorer_probs {
            model.get_or_create_player(player).goal = Some(player_goal_prob);
        }

        Ok(())
    }
}
