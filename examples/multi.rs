use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};
use tinyrand::StdRand;

use bentobox::capture::{Capture, CaptureMut};
use bentobox::mc;
use bentobox::probs::SliceExt;
use bentobox::selection::Selection;

fn main() {
    // probs taken from a popular website
    // let mut probs = vec![
    //     1.0 / 11.0,
    //     1.0 / 41.0,
    //     1.0 / 18.0,
    //     1.0 / 12.0,
    //     1.0 / 91.0,
    //     1.0 / 101.0,
    //     1.0 / 4.8,
    //     1.0 / 14.0,
    //     1.0 / 2.9,
    //     1.0 / 91.0,
    //     1.0 / 9.0,
    //     1.0 / 91.0,
    //     1.0 / 5.0,
    //     1.0 / 21.0,
    // ];
    let mut probs = vec![
        1.0 / 2.0,
        1.0 / 12.0,
        1.0 / 3.0,
        1.0 / 9.50,
        1.0 / 7.50,
        1.0 / 126.0,
        1.0 / 23.0,
        1.0 / 14.0,
    ];

    // force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
    let overround = probs.normalize();

    println!("fair probs: {probs:?}");
    println!("overround: {overround:.3}");

    // create an MC engine for reuse
    let podium_places = 4;
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(100_000)
        .with_probabilities(Capture::Borrowed(&probs))
        .with_podium_places(podium_places)
        .with_rand(CaptureMut::Owned(StdRand::default()));

    // simulate top-N rankings for all runners
    // NOTE: rankings and runner numbers are zero-based
    let mut data = vec![];
    for runner in 0..probs.len() {
        let mut rank_data = vec![];
        for rank in 0..4 {
            let frac = engine.simulate(&vec![Selection::Span {
                runner,
                ranks: 0..rank + 1,
            }]);
            rank_data.push(frac);
        }
        data.push(rank_data);
    }

    let mut table = Table::default()
        .with_cols({
            let mut cols = vec![];
            cols.push(Col::new(
                Styles::default().with(MinWidth(10)).with(HAlign::Centred),
            ));
            for _ in 0..podium_places {
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
            for _ in 0..podium_places {
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
            for _ in 0..podium_places {
                cols.push(Col::new(
                    Styles::default().with(MinWidth(10)).with(HAlign::Right),
                ));
            }
            cols
        })
        .with_row({
            let mut header_cells = vec!["".into()];
            header_cells.push("Probability".into());
            for _ in 0..podium_places {
                header_cells.push("".into());
            }
            header_cells.push("Fair price".into());
            for _ in 0..podium_places {
                header_cells.push("".into());
            }
            header_cells.push("Market odds".into());
            for _ in 1..podium_places {
                header_cells.push("".into());
            }
            Row::new(
                Styles::default().with(Header(true)).with(Separator(true)),
                header_cells.into(),
            )
        })
        .with_row({
            let mut header_cells = vec!["Runner".into()];
            for rank in 0..podium_places {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            header_cells.push("".into());
            for rank in 0..podium_places {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            header_cells.push("".into());
            for rank in 0..podium_places {
                header_cells.push(format!("Top-{}", rank + 1).into());
            }
            Row::new(Styles::default().with(Header(true)), header_cells.into())
        });

    for (runner, rank_data) in data.iter().enumerate() {
        //println!("runner: {runner}");
        let mut row_cells = vec![format!("{}", runner + 1).into()];
        for frac in rank_data {
            row_cells.push(format!("{}", frac.quotient()).into());
        }
        row_cells.push(format!("{}", runner + 1).into());
        for frac in rank_data {
            row_cells.push(format!("{:.3}", 1.0 / frac.quotient()).into());
        }
        row_cells.push(format!("{}", runner + 1).into());
        for frac in rank_data {
            let odds = f64::max(1.0, 1.0 / frac.quotient() / overround);
            row_cells.push(format!("{odds:.3}").into());
        }
        table.push_row(Row::new(Styles::default(), row_cells.into()));
    }
    println!("{}", Console::default().render(&table));

    // simulate a same-race multi for a chosen selection vector
    let selections = vec![
        Selection::Span {
            runner: 0,
            ranks: 0..1,
        },
        Selection::Span {
            runner: 1,
            ranks: 0..2,
        },
        Selection::Span {
            runner: 2,
            ranks: 0..3,
        },
    ];
    let frac = engine.simulate(&selections);
    println!(
        "probability of {selections:?}: {}, fair price: {:.3}, market odds: {:.3}",
        frac.quotient(),
        1.0 / frac.quotient(),
        1.0 / frac.quotient() / overround.powi(selections.len() as i32)
    );
}
