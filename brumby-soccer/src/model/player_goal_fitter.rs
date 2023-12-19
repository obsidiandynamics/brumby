use rustc_hash::FxHashMap;
use crate::domain::{Offer, OfferType};
use crate::domain::error::OfferCapture;
use crate::fit;
use crate::interval::BivariateProbs;
use crate::model::{FitError, get_offer, Model};

pub struct PlayerAssistFitter;
impl PlayerAssistFitter {
    pub fn fit(&self, model: &mut Model, offers: &FxHashMap<OfferType, Offer>) -> Result<(), FitError> {
        model.require_team_goal_probs()?;
        let first_gs = OfferCapture::try_from(get_offer(offers, &OfferType::FirstGoalscorer)?)?;
        first_gs.validate()?;
        // let fitted_goalscorer_probs = fit::fit_first_goalscorer_all(
        //     &BivariateProbs::from(adj_optimal_h1.as_slice()),
        //     &BivariateProbs::from(adj_optimal_h2.as_slice()),
        //     &first_gs,
        //     draw_prob,
        //     INTERVALS,
        //     MAX_TOTAL_GOALS_FULL
        // );

        Ok(())
    }
}