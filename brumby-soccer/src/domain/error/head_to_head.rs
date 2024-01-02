use brumby::hash_lookup::HashLookup;

use crate::domain::{DrawHandicap, error, OfferType, Outcome, Side, WinHandicap};
use crate::domain::error::{ExtraneousOutcome, InvalidOffer, InvalidOutcome};

// pub(crate) fn validate_outcomes(
//     offer_type: &OfferType,
//     outcomes: &HashLookup<Outcome>,
// ) -> Result<(), InvalidOutcome> {
//     match offer_type {
//         OfferType::HeadToHead(_, draw_handicap) => {
//             error::OutcomesCompleteAssertion {
//                 outcomes: &valid_outcomes(draw_handicap),
//             }
//             .check(outcomes, offer_type)?;
//             Ok(())
//         }
//         _ => unreachable!()
//     }
// }
pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<Outcome>,
    draw_handicap: &DrawHandicap
) -> Result<(), InvalidOutcome> {
    error::OutcomesCompleteAssertion {
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

// pub(crate) fn validate_outcome(
//     offer_type: &OfferType,
//     outcome: &Outcome,
// ) -> Result<(), InvalidOutcome> {
//     match offer_type {
//         OfferType::HeadToHead(_, draw_handicap) => {
//             let valid_outcomes = valid_outcomes(draw_handicap);
//             if valid_outcomes.contains(outcome) {
//                 Ok(())
//             } else {
//                 Err(InvalidOutcome::ExtraneousOutcome(ExtraneousOutcome {
//                     outcome: outcome.clone(),
//                     offer_type: offer_type.clone(),
//                 }))
//             }
//         }
//         _ => unreachable!()
//     }
// }

pub(crate) fn validate_probs(offer_type: &OfferType, probs: &[f64]) -> Result<(), InvalidOffer> {
    error::BooksumAssertion::with_default_tolerance(1.0..=1.0).check(probs, offer_type)?;
    Ok(())
}

fn to_win_handicap(draw_handicap: &DrawHandicap) -> WinHandicap {
    match draw_handicap {
        DrawHandicap::Ahead(by) => WinHandicap::AheadOver(*by),
        DrawHandicap::Behind(by) => WinHandicap::BehindUnder(*by)
    }
}

fn valid_outcomes(draw_handicap: &DrawHandicap) -> [Outcome; 3] {
    [
        Outcome::Win(Side::Home, to_win_handicap(draw_handicap)),
        Outcome::Win(Side::Away, to_win_handicap(draw_handicap).flip()),
        Outcome::Draw(draw_handicap.clone()),
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, Period};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(2));
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
                Outcome::Draw(DrawHandicap::Ahead(2)),
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
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
                Outcome::Draw(DrawHandicap::Ahead(2)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.4, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for HeadToHead(FullTime, Ahead(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::Win(Side::Home, WinHandicap::AheadOver(2)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(2)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
        };
        assert_eq!(
            "Draw(Ahead(2)) missing from HeadToHead(FullTime, Ahead(2))",
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
                Outcome::None,
            ]),
            market: Market::frame(
                &Overround::fair(),
                vec![0.4, 0.5, 0.05, 0.05],
                &PRICE_BOUNDS,
            ),
        };
        assert_eq!(
            "None does not belong in HeadToHead(FullTime, Ahead(2))",
            offer.validate().unwrap_err().to_string()
        );
    }
}
