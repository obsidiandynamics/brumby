//! Utilities for working with Stanza tables.

use stanza::renderer::{RenderHint, Renderer};
use stanza::style::{Styled, Styles};
use stanza::table::{Cell, Col, Row, Table};

struct NopRenderer;

impl Renderer for NopRenderer {
    type Output = String;

    fn render_with_hints(&self, _: &Table, _: &[RenderHint]) -> Self::Output {
        unimplemented!()
    }
}

fn render_cell(cell: &Cell) -> String {
    cell.data().render(&NopRenderer).to_string()
}

pub fn merge(tables: &[Table]) -> Table {
    assert!(tables.len() >= 2, "at least two tables must be merged");
    let first = &tables[0];
    for other in tables.iter().skip(1) {
        assert_eq!(first.num_rows(), other.num_rows());
    }

    Table::with_styles(first.styles().clone())
        .with_cols({
            let mut cols = vec![];
            for col_index in 0..first.num_cols() {
                cols.push(Col::new(first.col(col_index).unwrap().styles().clone()));
            }
            for other in tables.iter().skip(1) {
                for col_index in 1..other.num_cols() {
                    cols.push(Col::new(other.col(col_index).unwrap().styles().clone()));
                }
            }
            cols
        })
        .with_rows({
            let mut rows = vec![];
            for row_index in 0..first.num_rows() {
                let mut row: Vec<Cell> = vec![];
                for col_index in 0..first.num_cols() {
                    row.push(render_cell(first.cell(col_index, row_index).unwrap()).into());
                }

                for other in tables.iter().skip(1) {
                    for col_index in 1..other.num_cols() {
                        row.push(render_cell(other.cell(col_index, row_index).unwrap()).into());
                    }
                }

                rows.push(Row::new(Styles::default(), row));
            }
            rows
        })
}
