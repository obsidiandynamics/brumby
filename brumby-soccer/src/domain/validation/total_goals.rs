use brumby::hash_lookup::HashLookup;

use crate::domain::{validation, OfferType, Outcome, Over};
use crate::domain::validation::{ExtraneousOutcome, InvalidOffer, InvalidOutcome};

// pub(crate) fn validate_outcomes(
//     offer_type: &OfferType,
//     outcomes: &HashLookup<Outcome>,
// ) -> Result<(), InvalidOutcome> {
//     match offer_type {
//         OfferType::TotalGoals(_, over) => {
//             error::OutcomesCompleteAssertion {
//                 outcomes: &valid_outcomes(over),
//             }
//             .check(outcomes, offer_type)?;
//             Ok(())
//         }
//         _ => unreachable!(),
//     }
// }

pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<Outcome>,
    over: &Over,
) -> Result<(), InvalidOutcome> {
    validation::OutcomesCompleteAssertion {
        outcomes: &valid_outcomes(over),
    }
    .check(outcomes, offer_type)?;
    Ok(())
}

// pub(crate) fn validate_outcome(
//     offer_type: &OfferType,
//     outcome: &Outcome,
// ) -> Result<(), InvalidOutcome> {
//     match offer_type {
//         OfferType::TotalGoals(_, over) => {
//             let valid_outcomes = valid_outcomes(over);
//             if valid_outcomes.contains(outcome) {
//                 Ok(())
//             } else {
//                 Err(InvalidOutcome::ExtraneousOutcome(ExtraneousOutcome {
//                     outcome: outcome.clone(),
//                     offer_type: offer_type.clone(),
//                 }))
//             }
//         }
//         _ => unreachable!(),
//     }
// }

pub(crate) fn validate_outcome(
    offer_type: &OfferType,
    outcome: &Outcome,
    over: &Over,
) -> Result<(), InvalidOutcome> {
    let valid_outcomes = valid_outcomes(over);
    if valid_outcomes.contains(outcome) {
        Ok(())
    } else {
        Err(InvalidOutcome::ExtraneousOutcome(ExtraneousOutcome {
            outcome: outcome.clone(),
            offer_type: offer_type.clone(),
        }))
    }
}

pub(crate) fn validate_probs(offer_type: &OfferType, probs: &[f64]) -> Result<(), InvalidOffer> {
    match offer_type {
        OfferType::TotalGoals(_, _) => {
            validation::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

fn valid_outcomes(over: &Over) -> [Outcome; 2] {
    [
        Outcome::Over(over.0), Outcome::Under(over.0 + 1),
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
            outcomes: HashLookup::from(vec![Outcome::Over(2), Outcome::Under(3)]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        offer.validate().unwrap();
    }

    #[test]
    fn wrong_booksum() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![Outcome::Over(2), Outcome::Under(3)]),
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
            outcomes: HashLookup::from(vec![Outcome::Over(2)]),
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
                Outcome::Over(2),
                Outcome::Under(3),
                Outcome::None,
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!(
            "None does not belong in TotalGoals(FullTime, Over(2))",
            offer.validate().unwrap_err().to_string()
        );
    }
}
