use crate::domain::{Offer, OfferType};
use crate::fit::FittingErrors;
use stanza::style::HAlign::Left;
use stanza::style::{HAlign, Header, MinWidth, Styles};
use stanza::table::{Col, Row, Table};

pub fn tabulate_offer(offer: &Offer) -> Table {
    Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(Left)),
            Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Outcome".into(), "Odds".into()],
        ))
        .with_rows(
            offer
                .outcomes
                .items()
                .iter()
                .enumerate()
                .map(|(index, outcome)| {
                    Row::new(
                        Styles::default(),
                        vec![
                            format!("{outcome:?}").into(),
                            format!("{:.2}", offer.market.prices[index]).into(),
                        ],
                    )
                }),
        )
}

pub fn tabulate_errors(errors: &[(&OfferType, FittingErrors)]) -> Table {
    Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(Left)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Offer type".into(), "RMSRE".into(), "RMSE".into()],
        ))
        .with_rows(errors.iter().map(|(offer_type, error)| {
            Row::new(
                Styles::default(),
                vec![
                    format!("{:?}", offer_type).into(),
                    format!("{:.3}", error.rmsre).into(),
                    format!("{:.3}", error.rmse).into(),
                ],
            )
        }))
}

pub fn tabulate_overrounds(offers: &[&Offer]) -> Table {
    Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(Left)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
            Col::new(Styles::default().with(MinWidth(5)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec![
                "Offer type".into(),
                "Overround".into(),
                "Outcomes".into(),
                "Increment".into(),
            ],
        ))
        .with_rows(offers.iter().map(|offer| {
            Row::new(
                Styles::default(),
                vec![
                    format!("{:?}", offer.offer_type).into(),
                    format!("{:.3}", offer.market.overround.value).into(),
                    format!("{}", offer.outcomes.len()).into(),
                    format!(
                        "{:.3}",
                        (offer.market.overround.value - 1.0) / offer.outcomes.len() as f64
                    )
                    .into(),
                ],
            )
        }))
}
