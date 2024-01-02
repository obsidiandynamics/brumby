use super::*;
use crate::domain::{DrawHandicap, Over, Period, Side, WinHandicap};
use brumby::market::{Market, Overround};

const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

#[test]
fn aligned_offer() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([Outcome::Over(2), Outcome::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    assert!(offer.validate().is_ok());
}

#[test]
fn misaligned_offer() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([Outcome::Over(2), Outcome::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4], &PRICE_BOUNDS),
    };
    assert_eq!(
        "2:1 outcomes:probabilities mapped for TotalGoals(FullTime, Over(2))",
        offer.validate().unwrap_err().to_string()
    );
}

#[test]
fn booksum_within_expected() {
    let assertion = BooksumAssertion {
        expected: 1.0..=2.0,
        tolerance: 0.01,
    };
    assertion
        .check(&[1.0 - 0.01], &OfferType::FirstGoalscorer)
        .unwrap();
    assertion
        .check(&[2.0 + 0.01], &OfferType::FirstGoalscorer)
        .unwrap();
}

#[test]
fn booksum_outside_expected() {
    let assertion = BooksumAssertion {
        expected: 1.0..=2.0,
        tolerance: 0.01,
    };
    {
        let err = assertion
            .check(&[1.0 - 0.011], &OfferType::FirstGoalscorer)
            .unwrap_err();
        assert_eq!(
            "expected booksum in 1.0..=2.0 ± 0.01, got 0.989 for FirstGoalscorer",
            err.to_string()
        );
    }
    {
        let err = assertion
            .check(&[2.0 + 0.011], &OfferType::FirstGoalscorer)
            .unwrap_err();
        assert_eq!(
            "expected booksum in 1.0..=2.0 ± 0.01, got 2.011 for FirstGoalscorer",
            err.to_string()
        );
    }
}

#[test]
fn alignment_correct() {
    OfferAlignmentAssertion::check(
        &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))],
        &[0.5, 0.5],
        &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
    )
    .unwrap();
}

#[test]
fn alignment_incorrect() {
    let err = OfferAlignmentAssertion::check(
        &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0))],
        &[0.5, 0.5],
        &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
    )
    .unwrap_err();
    assert_eq!(
        "1:2 outcomes:probabilities mapped for DrawNoBet(Ahead(0))",
        err.to_string()
    );
}

#[test]
fn outcomes_intact() {
    let assertion = OutcomesIntactAssertion {
        outcomes: &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))],
    };
    assertion
        .check(
            &HashLookup::from(vec![Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))]),
            &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
        )
        .unwrap();
}

#[test]
fn outcome_missing() {
    let assertion = OutcomesIntactAssertion {
        outcomes: &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))],
    };
    let err = assertion
        .check(
            &HashLookup::from([Outcome::Win(Side::Home, WinHandicap::AheadOver(0))]),
            &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
        )
        .unwrap_err();
    assert_eq!("Win(Away, BehindUnder(0)) missing from DrawNoBet(Ahead(0))", err.to_string());
}

#[test]
fn outcomes_match() {
    let mut assertion = OutcomesMatchAssertion {
        matcher: |outcome| matches!(outcome, Outcome::Win(_, _) | Outcome::Draw(_)),
    };
    assertion
        .check(
            &[
                Outcome::Win(Side::Home, WinHandicap::AheadOver(0)),
                Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)),
                Outcome::Draw(DrawHandicap::Ahead(0)),
            ],
            &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
        )
        .unwrap();
}

#[test]
fn outcome_does_not_match() {
    let mut assertion = OutcomesMatchAssertion {
        matcher: |outcome| matches!(outcome, Outcome::Win(_, _) | Outcome::Draw(_)),
    };
    let err = assertion
        .check(
            &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::None],
            &OfferType::HeadToHead(Period::FullTime, DrawHandicap::Ahead(0)),
        )
        .unwrap_err();
    assert_eq!(
        "None does not belong in HeadToHead(FullTime, Ahead(0))",
        err.to_string()
    );
}

#[test]
fn outcomes_complete() {
    let assertion = OutcomesCompleteAssertion {
        outcomes: &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))],
    };
    assertion
        .check(
            &HashLookup::from([Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))]),
            &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
        )
        .unwrap();
}

#[test]
fn outcomes_incomplete() {
    let assertion = OutcomesCompleteAssertion {
        outcomes: &[Outcome::Win(Side::Home, WinHandicap::AheadOver(0)), Outcome::Win(Side::Away, WinHandicap::BehindUnder(0))],
    };
    {
        let err = assertion
            .check(
                &HashLookup::from([Outcome::Win(Side::Home, WinHandicap::AheadOver(0))]),
                &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
            )
            .unwrap_err();
        assert_eq!("Win(Away, BehindUnder(0)) missing from DrawNoBet(Ahead(0))", err.to_string());
    }
    {
        let err = assertion
            .check(
                &HashLookup::from([
                    Outcome::Win(Side::Home, WinHandicap::AheadOver(0)),
                    Outcome::Win(Side::Away, WinHandicap::BehindUnder(0)),
                    Outcome::Draw(DrawHandicap::Ahead(0)),
                ]),
                &OfferType::DrawNoBet(DrawHandicap::Ahead(0)),
            )
            .unwrap_err();
        assert_eq!("Draw(Ahead(0)) does not belong in DrawNoBet(Ahead(0))", err.to_string());
    }
}

#[test]
fn unvalidated_offer_unchecked() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([Outcome::Over(2), Outcome::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert_eq!(
        OfferType::TotalGoals(Period::FullTime, Over(2)),
        unvalidated_offer.unchecked().offer_type
    );
}

#[test]
fn unvalidated_offer_unwrap_valid() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([Outcome::Over(2), Outcome::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert!(OfferCapture::try_from(unvalidated_offer).is_ok());
}

#[test]
fn unvalidated_offer_unwrap_invalid() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([Outcome::Over(2), Outcome::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert!(OfferCapture::try_from(unvalidated_offer).is_err());
}
