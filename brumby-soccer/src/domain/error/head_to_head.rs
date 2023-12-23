use brumby::hash_lookup::HashLookup;

use crate::domain::{error, OfferType, OutcomeType, Side};
use crate::domain::error::{ExtraneousOutcome, InvalidOffer, InvalidOutcome};

pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<OutcomeType>,
) -> Result<(), InvalidOutcome> {
    error::OutcomesCompleteAssertion {
        outcomes: &valid_outcomes(),
    }
    .check(outcomes, offer_type)?;
    Ok(())
}

pub(crate) fn validate_outcome(
    offer_type: &OfferType,
    outcome: &OutcomeType,
) -> Result<(), InvalidOutcome> {
    let valid_outcomes = valid_outcomes();
    if valid_outcomes.contains(outcome) {
        Ok(())
    } else {
        Err(InvalidOutcome::ExtraneousOutcome(ExtraneousOutcome {
            outcome_type: outcome.clone(),
            offer_type: offer_type.clone(),
        }))
    }
}

pub(crate) fn validate_probs(offer_type: &OfferType, probs: &[f64]) -> Result<(), InvalidOffer> {
    error::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
    Ok(())
}

fn valid_outcomes() -> [OutcomeType; 3] {
    [
        OutcomeType::Win(Side::Home),
        OutcomeType::Win(Side::Away),
        OutcomeType::Draw,
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, Period};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::HeadToHead(Period::FullTime);
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
                OutcomeType::Draw,
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.4, 0.2], &PRICE_BOUNDS),
        };
        offer.validate().unwrap();
    }

    #[test]
    fn wrong_booksum() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
                OutcomeType::Draw,
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.4, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for HeadToHead(FullTime)",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        assert_eq!(
            "Draw missing from HeadToHead(FullTime)",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
                OutcomeType::Draw,
                OutcomeType::None,
            ]),
            market: Market::frame(
                &Overround::fair(),
                vec![0.4, 0.5, 0.05, 0.05],
                &PRICE_BOUNDS,
            ),
        };
        assert_eq!(
            "None does not belong in HeadToHead(FullTime)",
            offer.validate().unwrap_err().to_string()
        );
    }
}
