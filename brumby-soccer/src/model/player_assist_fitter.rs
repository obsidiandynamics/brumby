use rustc_hash::FxHashMap;
use tracing::debug;

use crate::domain::error::{MissingOutcome, OfferCapture};
use crate::domain::{Offer, OfferType, OutcomeType};
use crate::fit;
use crate::interval::UnivariateProbs;
use crate::model::{get_offer, FitError, Model, get_or_create_player};

pub struct PlayerAssistFitter;
impl PlayerAssistFitter {
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

        let anytime_goalscorer =
            OfferCapture::try_from(get_offer(offers, &OfferType::AnytimeGoalscorer)?)?;
        let anytime_assist = OfferCapture::try_from(get_offer(offers, &OfferType::AnytimeAssist)?)?;
        let home_goalscorer_booksum = fit::home_booksum(&anytime_goalscorer);
        let away_goalscorer_booksum = fit::away_booksum(&anytime_goalscorer);
        // println!("partial goalscorer booksums: home: {home_goalscorer_booksum:.3}, away: {away_goalscorer_booksum:.3}");

        let home_assister_booksum = fit::home_booksum(&anytime_assist);
        let away_assister_booksum = fit::away_booksum(&anytime_assist);
        // println!("partial assister booksums: home: {home_assister_booksum:.3}, away: {away_assister_booksum:.3}");
        let assist_probs = UnivariateProbs {
            home: home_assister_booksum / home_goalscorer_booksum,
            away: away_assister_booksum / away_goalscorer_booksum,
        };
        debug!("team assist probs: {assist_probs:?}");

        let fitted_assist_probs = fit::fit_anytime_assist_all(
            &goal_probs.h1,
            &goal_probs.h2,
            &assist_probs,
            &anytime_assist,
            nil_all_draw_prob,
            anytime_assist.market.fair_booksum(),
            model.config.intervals,
            model.config.max_total_goals,
        );
        model.assist_probs = Some(assist_probs);

        for (player, player_assist_prob) in fitted_assist_probs {
            get_or_create_player(&mut model.player_probs, player).assist = Some(player_assist_prob);
        }

        Ok(())
    }
}
