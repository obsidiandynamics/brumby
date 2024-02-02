use brumby::hash_lookup::HashLookup;

use crate::domain::validation::{ExtraneousOutcome, InvalidOffer, InvalidOfferType, InvalidOutcome};
use crate::domain::{validation, DrawHandicap, OfferType, Outcome, Side, WinHandicap};

pub(crate) fn validate_outcomes(
    offer_type: &OfferType,
    outcomes: &HashLookup<Outcome>,
    draw_handicap: &DrawHandicap,
    win_handicap: &WinHandicap,
) -> Result<(), InvalidOutcome> {
    validation::OutcomesCompleteAssertion {
        outcomes: &valid_outcomes(draw_handicap, win_handicap),
    }
    .check(outcomes, offer_type)?;
    Ok(())
}

pub(crate) fn validate_outcome(
    offer_type: &OfferType,
    outcome: &Outcome,
    draw_handicap: &DrawHandicap,
    win_handicap: &WinHandicap,
) -> Result<(), InvalidOutcome> {
    let valid_outcomes = valid_outcomes(draw_handicap, win_handicap);
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
    win_handicap: &WinHandicap,
) -> Result<(), InvalidOfferType> {
    let verify_that = |condition| {
        if condition {
            Ok(())
        } else {
            Err(InvalidOfferType {
                offer_type: offer_type.clone(),
            }.into())
        }
    };

    match (draw_handicap, win_handicap) {
        (DrawHandicap::Ahead(ahead), WinHandicap::AheadOver(ahead_over)) => {
            if ahead == ahead_over {
                // -x.25 case
                Ok(())
            } else {
                // -x.75 case
                verify_that(*ahead == ahead_over + 1)
            }
        }
        (_, WinHandicap::BehindUnder(behind_under)) => {
            let behind = match draw_handicap {
                DrawHandicap::Ahead(0) => 0, // Behind(0) is always written as Ahead(0) by convention
                DrawHandicap::Behind(by) => *by,
                _ => unreachable!(),
            };
            if behind == *behind_under {
                // +x.75 case
                Ok(())
            } else {
                // +x.25 case
                verify_that(behind + 1 == *behind_under)
            }
        }
        _ => verify_that(false),
    }
}

fn valid_outcomes(draw_handicap: &DrawHandicap, win_handicap: &WinHandicap) -> [Outcome; 2] {
    [
        Outcome::SplitWin(Side::Home, draw_handicap.clone(), win_handicap.clone()),
        Outcome::SplitWin(Side::Away, draw_handicap.flip(), win_handicap.flip_asian()),
    ]
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use brumby::hash_lookup::HashLookup;
    use brumby::market::{Market, Overround};

    use crate::domain::{Offer, Period, WinHandicap};

    use super::*;

    const OFFER_TYPE: OfferType = OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2));
    const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

    #[test]
    fn valid() {
        {
            // -2 with -2.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),       // -2.25
                    Outcome::SplitWin(Side::Away, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),    // +2.25
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            offer.validate().unwrap();
        }
        {
            // -3 with -2.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Ahead(3), WinHandicap::AheadOver(2)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(3), WinHandicap::AheadOver(2)),       // -2.75
                    Outcome::SplitWin(Side::Away, DrawHandicap::Behind(3), WinHandicap::BehindUnder(3)),    // +2.75
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            offer.validate().unwrap();
        }
        {
            // +2 with +2.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),    // +2.25
                    Outcome::SplitWin(Side::Away, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),       // -2.25
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            offer.validate().unwrap();
        }
        {
            // +3 with +2.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Behind(3), WinHandicap::BehindUnder(3)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Behind(3), WinHandicap::BehindUnder(3)),    // +2.75
                    Outcome::SplitWin(Side::Away, DrawHandicap::Ahead(3), WinHandicap::AheadOver(2)),       // -2.75
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            offer.validate().unwrap();
        }
    }

    #[test]
    fn wrong_booksum() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),       // -2.25
                Outcome::SplitWin(Side::Away, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),    // +2.25
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5], &PRICE_BOUNDS),
        };
        assert_eq!(
            "expected booksum in 1.0..=1.0 ± 0.001, got 0.9 for SplitHandicap(FullTime, Ahead(2), AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn missing_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2))]),
            market: Market::frame(&Overround::fair(), vec![1.0], &PRICE_BOUNDS),
        };
        assert_eq!(
            "SplitWin(Away, Behind(2), BehindUnder(3)) missing from SplitHandicap(FullTime, Ahead(2), AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn extraneous_outcome() {
        let offer = Offer {
            offer_type: OFFER_TYPE,
            outcomes: HashLookup::from(vec![
                Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),       // -2.25
                Outcome::SplitWin(Side::Away, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),    // +2.25
                Outcome::Draw(DrawHandicap::Ahead(2)),
            ]),
            market: Market::frame(&Overround::fair(), vec![0.4, 0.5, 0.1], &PRICE_BOUNDS),
        };
        assert_eq!(
            "Draw(Ahead(2)) does not belong in SplitHandicap(FullTime, Ahead(2), AheadOver(2))",
            offer.validate().unwrap_err().to_string()
        );
    }

    #[test]
    fn invalid_type() {
        {
            // -2 cannot be mixed with -3.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Ahead(2), WinHandicap::AheadOver(3)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(3)),
                    Outcome::SplitWin(Side::Away, DrawHandicap::Behind(2), WinHandicap::BehindUnder(3)),
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            assert_eq!(
                "SplitHandicap(FullTime, Ahead(2), AheadOver(3)) is not a valid offer type",
                offer.validate().unwrap_err().to_string()
            );
        }
        {
            // +2 cannot be mixed with +0.5
            let offer = Offer {
                offer_type: OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Behind(2), WinHandicap::BehindUnder(1)),
                outcomes: HashLookup::from(vec![
                    Outcome::SplitWin(Side::Home, DrawHandicap::Ahead(2), WinHandicap::AheadOver(2)),
                    Outcome::SplitWin(Side::Away, DrawHandicap::Behind(2), WinHandicap::BehindUnder(1)),
                ]),
                market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
            };
            assert_eq!(
                "SplitHandicap(FullTime, Behind(2), BehindUnder(1)) is not a valid offer type",
                offer.validate().unwrap_err().to_string()
            );
        }
    }
}
