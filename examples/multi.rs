use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use stanza::style::{HAlign, Header, MinWidth, Separator, Styles};
use stanza::table::{Col, Row, Table};

use bentobox::capture::Capture;
use bentobox::linear::Matrix;
use bentobox::mc;
use bentobox::mc::DilatedProbs;
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
    // let mut probs = vec![
    //     1.0 / 2.0,
    //     1.0 / 12.0,
    //     1.0 / 3.0,
    //     1.0 / 9.50,
    //     1.0 / 7.50,
    //     1.0 / 126.0,
    //     1.0 / 23.0,
    //     1.0 / 14.0,
    // ];
    // let mut probs = vec![
    //     1.0 / 3.7,
    //     1.0 / 14.0,
    //     1.0 / 5.50,
    //     1.0 / 9.50,
    //     1.0 / 1.90,
    //     1.0 / 13.0,
    // ];
    let mut win_probs = vec![
        1.0 / 1.55,
        1.0 / 12.0,
        1.0 / 6.50,
        1.0 / 9.00,
        1.0 / 9.00,
        1.0 / 61.0,
        1.0 / 7.5,
        1.0 / 81.0,
    ];

    // force probs to sum to 1 and extract the approximate overround used (multiplicative method assumed)
    let overround = win_probs.normalise(1.0);

    //TODO fav-longshot bias removal
    let favlong_dilate = -0.0;
    win_probs.dilate_power(favlong_dilate);

    // let dilatives = [0.0, 0.20, 0.35, 0.5];
    let dilatives = [0.0, 0.50, 0.50, 0.50];
    // let dilatives = [0.0, 0.0, 0.0, 0.0];

    let ranked_overrounds = [overround, 1.239, 1.169, 1.12];

    println!("win probs: {win_probs:?}");
    println!("dilatives: {dilatives:?}");
    println!("overround: {overround:.3}");

    let dilated_probs: Matrix<_> =  DilatedProbs::default()
        .with_win_probs(Capture::Borrowed(&win_probs))
        .with_dilatives(Capture::Borrowed(&dilatives))
        .into();

    println!("rank-runner probabilities: \n{}", dilated_probs.verbose());

    // create an MC engine for reuse
    let mut engine = mc::MonteCarloEngine::default()
        .with_iterations(1_000_000)
        .with_probs(Capture::Owned(dilated_probs));

    // simulate top-N rankings for all runners
    // NOTE: rankings and runner numbers are zero-based
    let podium_places = dilatives.len();
    let mut derived = Matrix::allocate(podium_places, win_probs.len());
    for runner in 0..win_probs.len() {
        for rank in 0..4 {
            let frac = engine.simulate(&vec![Selection::Span {
                runner,
                ranks: 0..rank + 1,
            }]);
            derived[(rank, runner)] = frac.quotient();
        }
    }

    for row in 0..derived.rows() {
        // println!("sum for row {row}: {}", derived.row_slice(row).sum());
        derived.row_slice_mut(row).normalise(row as f64 + 1.0);
    }

    //TODO fav-longshot bias addition
    derived.row_slice_mut(0).dilate_power(1.0/(1.0 + favlong_dilate) - 1.0);

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

    for runner in 0..win_probs.len() {
        //println!("runner: {runner}");
        let mut row_cells = vec![format!("{}", runner + 1).into()];
        for rank in 0..podium_places {
            row_cells.push(format!("{:.6}", derived[(rank, runner)]).into());
        }
        row_cells.push(format!("{}", runner + 1).into());
        for rank in 0..podium_places {
            row_cells.push(format!("{:.3}", 1.0 / derived[(rank, runner)]).into());
        }
        row_cells.push(format!("{}", runner + 1).into());
        for rank in 0..podium_places {
            let odds = f64::max(1.04, 1.0 / derived[(rank, runner)] / ranked_overrounds[rank]);
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
