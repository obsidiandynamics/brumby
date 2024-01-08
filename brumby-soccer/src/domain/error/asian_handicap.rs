use brumby::hash_lookup::HashLookup;

use crate::domain::{error, OfferType, Outcome, Side, WinHandicap};
use crate::domain::error::{ExtraneousOutcome, InvalidOffer, InvalidOutcome};

pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<Outcome>,
    win_handicap: &WinHandicap
) -> Result<(), InvalidOutcome> {
    error::OutcomesCompleteAssertion {
        outcomes: &valid_outcomes(win_handicap),
    }
    .check(outcomes, offer_type)?;
    Ok(())
}

pub(crate) fn validate_outcome(
    offer_type: &OfferType,
    outcome: &Outcome,
    win_handicap: &WinHandicap
) -> Result<(), InvalidOutcome> {
    let valid_outcomes = valid_outcomes(win_handicap);
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
    error::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
    Ok(())
}

fn valid_outcomes(win_handicap: &WinHandicap) -> [Outcome; 2] {
    [
        Outcome::Win(Side::Home, win_handicap.clone()),
        Outcome::Win(Side::Away, win_handicap.flip_asian()),
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, Period, WinHandicap};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::AsianHandicap(Period::FullTime, WinHandicap::AheadOver(2));
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(3)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        offer.validate().unwrap();
    }

    #[test]
    fn wrong_booksum() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(3)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for AsianHandicap(FullTime, AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
            ]),
            market: Market::frame(&Overround::fair(), vec![1.0], &PRICE_BOUNDS),
        };
        assert_eq!(
            "Win(Away, BehindUnder(3)) missing from AsianHandicap(FullTime, AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(3)),
                Outcome::None,
            ]),
            market: Market::frame(
                &Overround::fair(),
                vec![0.4, 0.5, 0.1],
                &PRICE_BOUNDS,
            ),
        };
        assert_eq!(
            "None does not belong in AsianHandicap(FullTime, AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }
}
