use super::*;
use crate::domain::{Over, Period, Side};
use brumby::market::{Market, Overround};

const PRICE_BOUNDS: RangeInclusive<f64> = 1.0..=1001.0;

#[test]
fn aligned_offer() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    assert!(offer.validate().is_ok());
}

#[test]
fn misaligned_offer() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
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
        &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
        &[0.5, 0.5],
        &OfferType::DrawNoBet,
    )
        .unwrap();
}

#[test]
fn alignment_incorrect() {
    let err = OfferAlignmentAssertion::check(
        &[OutcomeType::Win(Side::Home)],
        &[0.5, 0.5],
        &OfferType::DrawNoBet,
    )
        .unwrap_err();
    assert_eq!(
        "1:2 outcomes:probabilities mapped for DrawNoBet",
        err.to_string()
    );
}

#[test]
fn outcomes_intact() {
    let assertion = OutcomesIntactAssertion {
        outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
    };
    assertion
        .check(
            &HashLookup::from(vec![
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
            ]),
            &OfferType::DrawNoBet,
        )
        .unwrap();
}

#[test]
fn outcome_missing() {
    let assertion = OutcomesIntactAssertion {
        outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
    };
    let err = assertion
        .check(
            &HashLookup::from([OutcomeType::Win(Side::Home)]),
            &OfferType::DrawNoBet,
        )
        .unwrap_err();
    assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
}

#[test]
fn outcomes_match() {
    let mut assertion = OutcomesMatchAssertion {
        matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
    };
    assertion
        .check(
            &[
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
                OutcomeType::Draw,
            ],
            &OfferType::HeadToHead(Period::FullTime),
        )
        .unwrap();
}

#[test]
fn outcome_does_not_match() {
    let mut assertion = OutcomesMatchAssertion {
        matcher: |outcome| matches!(outcome, OutcomeType::Win(_) | OutcomeType::Draw),
    };
    let err = assertion
        .check(
            &[OutcomeType::Win(Side::Home), OutcomeType::None],
            &OfferType::HeadToHead(Period::FullTime),
        )
        .unwrap_err();
    assert_eq!(
        "None does not belong in HeadToHead(FullTime)",
        err.to_string()
    );
}

#[test]
fn outcomes_complete() {
    let assertion = OutcomesCompleteAssertion {
        outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
    };
    assertion
        .check(
            &HashLookup::from([
                OutcomeType::Win(Side::Home),
                OutcomeType::Win(Side::Away),
            ]),
            &OfferType::DrawNoBet,
        )
        .unwrap();
}

#[test]
fn outcomes_incomplete() {
    let assertion = OutcomesCompleteAssertion {
        outcomes: &[OutcomeType::Win(Side::Home), OutcomeType::Win(Side::Away)],
    };
    {
        let err = assertion
            .check(
                &HashLookup::from([OutcomeType::Win(Side::Home)]),
                &OfferType::DrawNoBet,
            )
            .unwrap_err();
        assert_eq!("Win(Away) missing from DrawNoBet", err.to_string());
    }
    {
        let err = assertion
            .check(
                &HashLookup::from([
                    OutcomeType::Win(Side::Home),
                    OutcomeType::Win(Side::Away),
                    OutcomeType::Draw,
                ]),
                &OfferType::DrawNoBet,
            )
            .unwrap_err();
        assert_eq!("Draw does not belong in DrawNoBet", err.to_string());
    }
}

#[test]
fn unvalidated_offer_unchecked() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert_eq!(OfferType::TotalGoals(Period::FullTime, Over(2)), unvalidated_offer.unchecked().offer_type);
}

#[test]
fn unvalidated_offer_unwrap_valid() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4, 0.6], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert!(OfferCapture::try_from(unvalidated_offer).is_ok());
}

#[test]
fn unvalidated_offer_unwrap_invalid() {
    let offer = Offer {
        offer_type: OfferType::TotalGoals(Period::FullTime, Over(2)),
        outcomes: HashLookup::from([OutcomeType::Over(2), OutcomeType::Under(3)]),
        market: Market::frame(&Overround::fair(), vec![0.4], &PRICE_BOUNDS),
    };
    let unvalidated_offer = UnvalidatedOffer::from(Capture::from(offer));
    assert!(OfferCapture::try_from(unvalidated_offer).is_err());
}