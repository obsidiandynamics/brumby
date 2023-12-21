use rustc_hash::FxHashMap;
use tracing::debug;

use crate::domain::error::{MissingOutcome, OfferCapture};
use crate::domain::{Offer, OfferType, OutcomeType};
use crate::fit;
use crate::model::{get_offer, FitError, Model};

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

        for (player, player_goal_prob) in fitted_goalscorer_probs {
            model.get_or_create_player(player).goal = Some(player_goal_prob);
        }

        Ok(())
    }
}
