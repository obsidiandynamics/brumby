use brumby::hash_lookup::HashLookup;

use crate::domain::{error, OfferType, OutcomeType, Side};
use crate::domain::error::{InvalidOffer, InvalidOutcome};

pub fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<OutcomeType>,
) -> Result<(), InvalidOutcome> {
    match offer_type {
        OfferType::HeadToHead(_) => {
            error::OutcomesCompleteAssertion {
                outcomes: &valid_outcomes(),
            }
            .check(outcomes, offer_type)?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

pub fn validate_probs(offer_type: &OfferType, probs: &[f64]) -> Result<(), InvalidOffer> {
    match offer_type {
        OfferType::HeadToHead(_) => {
            error::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
            Ok(())
        }
        _ => unreachable!(),
    }
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
            "expected booksum in 1.0..=1.0 ± 0.000001, got 0.9 for HeadToHead(FullTime)",
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
