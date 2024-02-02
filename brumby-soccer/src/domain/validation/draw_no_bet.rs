use brumby::hash_lookup::HashLookup;

use crate::domain::{DrawHandicap, validation, OfferType, Outcome, Side};
use crate::domain::validation::{ExtraneousOutcome, InvalidOffer, InvalidOfferType, InvalidOutcome};

pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<Outcome>,
    draw_handicap: &DrawHandicap
) -> Result<(), InvalidOutcome> {
    validation::OutcomesCompleteAssertion {
        outcomes: &valid_outcomes(draw_handicap),
    }
    .check(outcomes, offer_type)?;
    Ok(())
}

pub(crate) fn validate_outcome(
    offer_type: &OfferType,
    outcome: &Outcome,
    draw_handicap: &DrawHandicap
) -> Result<(), InvalidOutcome> {
    let valid_outcomes = valid_outcomes(draw_handicap);
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
    validation::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
    Ok(())
}

pub(crate) fn validate_type(
    offer_type: &OfferType,
    draw_handicap: &DrawHandicap,
) -> Result<(), InvalidOfferType> {
    match draw_handicap {
        DrawHandicap::Behind(0) => Err(InvalidOfferType {
            offer_type: offer_type.clone(),
        }),
        _ => Ok(()),
    }
}

fn valid_outcomes(draw_handicap: &DrawHandicap) -> [Outcome; 2] {
    [
        Outcome::Win(Side::Home, draw_handicap.to_win_handicap()),
        Outcome::Win(Side::Away, draw_handicap.to_win_handicap().flip_european()),
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, WinHandicap};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::DrawNoBet(DrawHandicap::Ahead(2));
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
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
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for DrawNoBet(Ahead(2))",
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
            "Win(Away, BehindUnder(2)) missing from DrawNoBet(Ahead(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
                Outcome::Draw(DrawHandicap::Ahead(2)),
            ]),
            market: Market::frame(
                &Overround::fair(),
                vec![0.4, 0.5, 0.1],
                &PRICE_BOUNDS,
            ),
        };
        assert_eq!(
            "Draw(Ahead(2)) does not belong in DrawNoBet(Ahead(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn invalid_type() {
        let offer = Offer {
            offer_type: OfferType::DrawNoBet(DrawHandicap::Behind(0)),
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::BehindUnder(0)),
                Outcome::Win(Side::Away, WinHandicap::AheadOver(0)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        assert_eq!(
            "DrawNoBet(Behind(0)) is not a valid offer type",
            offer.validate().unwrap_err().to_string()
        );
    }
}
