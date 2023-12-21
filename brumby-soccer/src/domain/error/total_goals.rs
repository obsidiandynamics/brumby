use brumby::hash_lookup::HashLookup;

use crate::domain::error::{InvalidOffer, InvalidOutcome};
use crate::domain::{error, OfferType, OutcomeType, Over};

pub fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<OutcomeType>,
) -> Result<(), InvalidOutcome> {
    match offer_type {
        OfferType::TotalGoals(_, over) => {
            error::OutcomesCompleteAssertion {
                outcomes: &valid_outcomes(over),
            }
            .check(outcomes, offer_type)?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

pub fn validate_probs(offer_type: &OfferType, probs: &[f64]) -> Result<(), InvalidOffer> {
    match offer_type {
        OfferType::TotalGoals(_, _) => {
            error::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

// pub fn create_outcomes(offer_type: &OfferType) -> [OutcomeType; 2] {
//     match offer_type {
//         OfferType::TotalGoals(_, over) => _create_outcomes(over),
//         _ => unreachable!(),
//     }
// }

fn valid_outcomes(over: &Over) -> [OutcomeType; 2] {
    [
        OutcomeType::Over(over.0), OutcomeType::Under(over.0 + 1),
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, Over, Period};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::TotalGoals(Period::FullTime, Over(2));
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![OutcomeType::Over(2), OutcomeType::Under(3)]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        offer.validate().unwrap();
    }

    #[test]
    fn wrong_booksum() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![OutcomeType::Over(2), OutcomeType::Under(3)]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for TotalGoals(FullTime, Over(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![OutcomeType::Over(2)]),
            market: Market::frame(&Overround::fair(), vec![1.0], &PRICE_BOUNDS),
        };
        assert_eq!(
            "Under(3) missing from TotalGoals(FullTime, Over(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                OutcomeType::Over(2),
                OutcomeType::Under(3),
                OutcomeType::None,
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!(
            "None does not belong in TotalGoals(FullTime, Over(2))",
            offer.validate().unwrap_err().to_string()
        );
    }
}
