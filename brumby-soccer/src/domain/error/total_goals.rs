use crate::domain::{error, Offer, OfferType, OutcomeType};
use crate::domain::error::InvalidOffer;

pub fn validate(offer: &Offer) -> Result<(), InvalidOffer> {
    match &offer.offer_type {
        OfferType::TotalGoals(_, over) => {
            error::BooksumAssertion::with_default_tolerance(1.0..=1.0)
                .check(&offer.market.probs, &offer.offer_type)?;
            error::OutcomesCompleteAssertion {
                outcomes: &[OutcomeType::Over(over.0), OutcomeType::Under(over.0 + 1)],
            }
            .check(&offer.outcomes, &offer.offer_type)?;
            Ok(())
        }
        _ => panic!("{:?} unsupported", offer.offer_type),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Over, Period};

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
        assert_eq!("wrong booksum: expected 1.0..=1.0 ± 0.000001, got 0.9 for TotalGoals(FullTime, Over(2))", offer.validate().unwrap_err().to_string());
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![OutcomeType::Over(2)]),
            market: Market::frame(&Overround::fair(), vec![1.0], &PRICE_BOUNDS),
        };
        assert_eq!("Under(3) missing from TotalGoals(FullTime, Over(2))", offer.validate().unwrap_err().to_string());
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![OutcomeType::Over(2), OutcomeType::Under(3), OutcomeType::None]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!("None does not belong in TotalGoals(FullTime, Over(2))", offer.validate().unwrap_err().to_string());
    }
}
