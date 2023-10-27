use stanza::style::{HAlign, Header, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};
use crate::linear::Matrix;
use crate::probs::MarketPrice;
use crate::selection::Runner;

#[derive(Debug, Default)]
pub struct DerivedPrice {
    pub probability: f64,
    pub fair_price: f64,
    pub market_price: f64,
}

impl MarketPrice for DerivedPrice {
    fn decimal(&self) -> f64 {
        self.market_price
    }
}

pub fn tabulate(derived: &Matrix<DerivedPrice>) -> Table {
    let mut table = Table::default()
        .with_cols({
            let mut cols = vec![];
            cols.push(Col::new(
                Styles::default().with(MinWidth(10)).with(HAlign::Centred),
            ));
            for _ in 0..derived.rows() {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols.push(Col::new(
                Styles::default()
                    .with(Separator(true))
                    .with(MinWidth(5))
                    .with(HAlign::Centred),
            ));
            for _ in 0..derived.rows() {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols.push(Col::new(
                Styles::default()
                    .with(Separator(true))
                    .with(MinWidth(5))
                    .with(HAlign::Centred),
            ));
            for _ in 0..derived.rows() {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols
        })
        .with_row({
            let mut header_cells = vec!["".into()];
            header_cells.push("Probability".into());
            for _ in 0..derived.rows() {
                header_cells.push("".into());
            }
            header_cells.push("Fair price".into());
            for _ in 0..derived.rows() {
                header_cells.push("".into());
            }
            header_cells.push("Market odds".into());
            for _ in 1..derived.rows() {
                header_cells.push("".into());
            }
            Row::new(
                Styles::default().with(Header(true)).with(Separator(true)),
                header_cells,
            )
        })
        .with_row({
            let mut header_cells = vec!["Runner".into()];
            for rank in 0..derived.rows() {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            header_cells.push("".into());
            for rank in 0..derived.rows() {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            header_cells.push("".into());
            for rank in 0..derived.rows() {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            Row::new(Styles::default().with(Header(true)), header_cells)
        });

    for runner in 0..derived.cols() {
        let mut row_cells = vec![format!("{}", Runner::index(runner)).into()];
        for rank in 0..derived.rows() {
            row_cells.push(format!("{:.6}", derived[(rank, runner)].probability).into());
        }
        row_cells.push(format!("{}", Runner::index(runner)).into());
        for rank in 0..derived.rows() {
            row_cells.push(format!("{:.3}", derived[(rank, runner)].fair_price).into());
        }
        row_cells.push(format!("{}", Runner::index(runner)).into());
        for rank in 0..derived.rows() {
            row_cells.push(format!("{:.3}", derived[(rank, runner)].market_price).into());
        }
        table.push_row(Row::new(Styles::default(), row_cells));
    }

    table
}