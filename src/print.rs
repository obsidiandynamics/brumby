use stanza::style::{HAlign, Header, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};
use crate::linear::Matrix;
use crate::market::MarketPrice;
use crate::selection::{Rank, Runner};

#[derive(Debug, Default)]
pub struct DerivedPrice {
    pub probability: f64,
    pub price: f64,
}

impl MarketPrice for DerivedPrice {
    fn decimal(&self) -> f64 {
        self.price
    }
}

pub fn tabulate_derived_prices(derived: &Matrix<DerivedPrice>) -> Table {
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
            header_cells.push("Probabilities".into());
            for _ in 0..derived.rows() {
                header_cells.push("".into());
            }
            header_cells.push("Fair prices".into());
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
            row_cells.push(format!("{:.3}", 1.0 / derived[(rank, runner)].price).into());
        }
        row_cells.push(format!("{}", Runner::index(runner)).into());
        for rank in 0..derived.rows() {
            row_cells.push(format!("{:.3}", derived[(rank, runner)].price).into());
        }
        table.push_row(Row::new(Styles::default(), row_cells));
    }

    table
}

pub fn tabulate_values(values: &[f64], header: &str) -> Table {
    let mut table = Table::default()
        .with_cols(vec![
            Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Centred)),
            Col::new(Styles::default().with(MinWidth(10)).with(HAlign::Right)),
        ])
        .with_row(Row::new(
            Styles::default().with(Header(true)),
            vec!["Rank".into(), header.into()],
        ));
    for (rank, error) in values.iter().enumerate() {
        table.push_row(Row::new(
            Styles::default(),
            vec![
                format!("{}", Rank::index(rank)).into(),
                format!("{:.6}", error).into(),
            ],
        ));
    }
    table
}

pub fn tabulate_probs(probs: &Matrix<f64>) -> Table {
    let mut table = Table::default()
        .with_cols({
            let mut cols = vec![];
            cols.push(Col::new(
                Styles::default().with(MinWidth(10)).with(HAlign::Centred),
            ));
            for _ in 0..probs.rows() {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols
        })
        .with_row({
            let mut header_cells = vec!["Runner".into()];
            for rank in 0..probs.rows() {
                header_cells.push(format!("{}", Rank::index(rank)).into());
            }
            Row::new(Styles::default().with(Header(true)), header_cells)
        });

    for runner in 0..probs.cols() {
        let mut row_cells = vec![format!("{}", Runner::index(runner)).into()];
        for rank in 0..probs.rows() {
            row_cells.push(format!("{:.6}", probs[(rank, runner)]).into());
        }
        table.push_row(Row::new(Styles::default(), row_cells));
    }

    table
}

pub fn tabulate_prices(prices: &Matrix<f64>) -> Table {
    let mut table = Table::default()
        .with_cols({
            let mut cols = vec![];
            cols.push(Col::new(
                Styles::default().with(MinWidth(10)).with(HAlign::Centred),
            ));
            for _ in 0..prices.rows() {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols
        })
        .with_row({
            let mut header_cells = vec!["Runner".into()];
            for rank in 0..prices.rows() {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            Row::new(Styles::default().with(Header(true)), header_cells)
        });

    for runner in 0..prices.cols() {
        let mut row_cells = vec![format!("{}", Runner::index(runner)).into()];
        for rank in 0..prices.rows() {
            row_cells.push(format!("{:.3}", prices[(rank, runner)]).into());
        }
        table.push_row(Row::new(Styles::default(), row_cells));
    }

    table
}