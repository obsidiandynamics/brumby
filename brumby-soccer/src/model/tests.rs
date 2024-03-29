use crate::domain::{DrawHandicap, Offer, OfferType, Outcome, Period, Side, WinHandicap};
use crate::model::{Config, Model, Stub};
use crate::print;
use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use brumby_testing::assert_slice_f64_relative;
use rustc_hash::FxHashMap;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;

const SINGLE_PRICE_BOUNDS: PriceBounds = 1.001..=1001.0;
const OVERROUND: Overround = Overround {
    method: OverroundMethod::Multiplicative,
    value: 1.0,
};
const EPSILON: f64 = 1e-3;

fn create_test_model() -> Model {
    Model::try_from(Config {
        intervals: 8,
        max_total_goals: 8,
    })
    .unwrap()
}

fn insert_head_to_head(model: &mut Model, draw_handicap: DrawHandicap, fair_prices: Vec<f64>) {
    let outcomes = HashLookup::from(vec![
        Outcome::Win(Side::Home, draw_handicap.to_win_handicap()),
        Outcome::Draw(draw_handicap.clone()),
        Outcome::Win(Side::Away, draw_handicap.to_win_handicap().flip_european()),
    ]);
    model.insert_offer(Offer {
        offer_type: OfferType::HeadToHead(Period::FullTime, draw_handicap),
        outcomes,
        market: Market::fit(&OVERROUND.method, fair_prices, 1.0),
    });
}

fn insert_asian_handicap(model: &mut Model, win_handicap: WinHandicap, fair_prices: Vec<f64>) {
    let outcomes = HashLookup::from(vec![
        Outcome::Win(Side::Home, win_handicap.clone()),
        Outcome::Win(Side::Away, win_handicap.flip_asian()),
    ]);
    model.insert_offer(Offer {
        offer_type: OfferType::AsianHandicap(Period::FullTime, win_handicap),
        outcomes,
        market: Market::fit(&OverroundMethod::Multiplicative, fair_prices, 1.0),
    });
}

fn stub_split_handicap(draw_handicap: DrawHandicap, win_handicap: WinHandicap) -> Stub {
    let outcomes = HashLookup::from([
        Outcome::SplitWin(Side::Home, draw_handicap.clone(), win_handicap.clone()),
        Outcome::SplitWin(Side::Away, draw_handicap.flip(), win_handicap.flip_asian()),
    ]);
    Stub {
        offer_type: OfferType::SplitHandicap(Period::FullTime, draw_handicap, win_handicap),
        outcomes,
        normal: 1.0,
        overround: OVERROUND.clone(),
    }
}

fn stub_draw_no_bet(draw_handicap: DrawHandicap) -> Stub {
    let win_handicap = draw_handicap.to_win_handicap();
    let outcomes = HashLookup::from([
        Outcome::Win(Side::Home, win_handicap.clone()),
        Outcome::Win(Side::Away, win_handicap.flip_european()),
    ]);
    Stub {
        offer_type: OfferType::DrawNoBet(draw_handicap),
        outcomes,
        normal: 1.0,
        overround: OVERROUND.clone(),
    }
}

#[test]
pub fn split_handicap_evenly_matched() {
    let mut model = create_test_model();
    insert_head_to_head(&mut model, DrawHandicap::Ahead(1), vec![4.91, 4.88, 1.68]);    // 0:1
    insert_head_to_head(&mut model, DrawHandicap::Ahead(0), vec![2.44, 4.13, 2.85]);    // 0:0
    insert_head_to_head(&mut model, DrawHandicap::Behind(1), vec![1.53, 5.33, 6.15]);   // 1:0
    insert_asian_handicap(&mut model, WinHandicap::AheadOver(0), vec![2.44, 1.68]);     // -0.5/+0.5
    insert_asian_handicap(&mut model, WinHandicap::BehindUnder(1), vec![1.53, 2.85]);   // +0.5/-0.5

    model
        .derive(
            &[
                stub_split_handicap(DrawHandicap::Ahead(1), WinHandicap::AheadOver(0)),     // -0.75/+0.75
                stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::AheadOver(0)),     // -0.25/+0.25
                stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::BehindUnder(1)),   // +0.25/-0.25
                stub_split_handicap(DrawHandicap::Behind(1), WinHandicap::BehindUnder(1)),  // +0.75/-0.75
            ],
            &SINGLE_PRICE_BOUNDS,
        )
        .unwrap();
    print_offers(model.offers());

    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(0),
            WinHandicap::AheadOver(0),
        ),
        &[2.156, 1.865],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(1),
            WinHandicap::AheadOver(0),
        ),
        &[2.944, 1.514],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(0),
            WinHandicap::BehindUnder(1),
        ),
        &[1.659, 2.517],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Behind(1),
            WinHandicap::BehindUnder(1),
        ),
        &[1.392, 3.549],
    );
}

#[test]
pub fn split_handicap_home_advantage() {
    let mut model = create_test_model();
    insert_head_to_head(&mut model, DrawHandicap::Ahead(2), vec![7.02, 6.34, 1.42]);    // 0:2
    insert_head_to_head(&mut model, DrawHandicap::Ahead(1), vec![3.33, 4.54, 2.08]);    // 0:1
    insert_head_to_head(&mut model, DrawHandicap::Ahead(0), vec![1.92, 4.54, 3.84]);    // 0:0
    insert_asian_handicap(&mut model, WinHandicap::AheadOver(1), vec![3.33, 1.42]);     // -1.5/+1.5
    insert_asian_handicap(&mut model, WinHandicap::AheadOver(0), vec![1.92, 2.08]);     // -0.5/+0.5

    model
        .derive(
            &[
                stub_split_handicap(DrawHandicap::Ahead(2), WinHandicap::AheadOver(1)),     // -1.75/+1.75
                stub_split_handicap(DrawHandicap::Ahead(1), WinHandicap::AheadOver(1)),     // -1.25/+1.25
                stub_split_handicap(DrawHandicap::Ahead(1), WinHandicap::AheadOver(0)),     // -0.75/+0.75
                stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::AheadOver(0)),     // -0.25/+0.25
            ],
            &SINGLE_PRICE_BOUNDS,
        )
        .unwrap();
    print_offers(model.offers());

    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(2),
            WinHandicap::AheadOver(1),
        ),
        &[4.182, 1.314],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(1),
            WinHandicap::AheadOver(1),
        ),
        &[2.977, 1.506],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(1),
            WinHandicap::AheadOver(0),
        ),
        &[2.171, 1.854],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(0),
            WinHandicap::AheadOver(0),
        ),
        &[1.712, 2.405],
    );
}

#[test]
pub fn split_handicap_away_advantage() {
    let mut model = create_test_model();
    insert_head_to_head(&mut model, DrawHandicap::Ahead(0), vec![3.84, 4.54, 1.92]);    // 0:0
    insert_head_to_head(&mut model, DrawHandicap::Behind(1), vec![2.08, 4.54, 3.33]);   // 1:0
    insert_head_to_head(&mut model, DrawHandicap::Behind(2), vec![1.42, 6.34, 7.02]);   // 2:0
    insert_asian_handicap(&mut model, WinHandicap::BehindUnder(1), vec![2.08, 1.92]);   // +0.5/-0.5
    insert_asian_handicap(&mut model, WinHandicap::BehindUnder(2), vec![1.42, 3.33]);   // +1.5/-1.5

    model
        .derive(
            &[
                stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::BehindUnder(1)),   // +0.25/-0.25
                stub_split_handicap(DrawHandicap::Behind(1), WinHandicap::BehindUnder(1)),  // +0.75/-0.75
                stub_split_handicap(DrawHandicap::Behind(1), WinHandicap::BehindUnder(2)),  // +1.25/-1.25
                stub_split_handicap(DrawHandicap::Behind(2), WinHandicap::BehindUnder(2)),  // +1.75/-1.75
            ],
            &SINGLE_PRICE_BOUNDS,
        )
        .unwrap();
    print_offers(model.offers());

    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Ahead(0),
            WinHandicap::BehindUnder(1),
        ),
        &[2.405, 1.712],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Behind(1),
            WinHandicap::BehindUnder(1),
        ),
        &[1.854, 2.171],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Behind(1),
            WinHandicap::BehindUnder(2),
        ),
        &[1.506, 2.977],
    );
    assert_prices(
        model.offers(),
        &OfferType::SplitHandicap(
            Period::FullTime,
            DrawHandicap::Behind(2),
            WinHandicap::BehindUnder(2),
        ),
        &[1.314, 4.182],
    );
}

#[test]
pub fn draw_no_bet() {
    let mut model = create_test_model();
    insert_head_to_head(&mut model, DrawHandicap::Ahead(0), vec![1.0/0.25, 1.0/0.35, 1.0/0.4]);

    model
        .derive(
            &[
                stub_draw_no_bet(DrawHandicap::Ahead(0)),
            ],
            &SINGLE_PRICE_BOUNDS,
        )
        .unwrap();
    print_offers(model.offers());

    assert_prices(
        model.offers(),
        &OfferType::DrawNoBet(
            DrawHandicap::Ahead(0),
        ),
        &[2.6, 1.625],
    );
}

fn assert_prices(
    offers: &FxHashMap<OfferType, Offer>,
    offer_type: &OfferType,
    expected_prices: &[f64],
) {
    let offer = offers.get(offer_type).unwrap();
    assert_slice_f64_relative(expected_prices, &offer.market.prices, EPSILON);
}

fn print_offers(offers: &FxHashMap<OfferType, Offer>) {
    for (_, offer) in sort_tuples(offers) {
        let table = print::tabulate_offer(offer);
        println!(
            "{:?}:\n{}",
            offer.offer_type,
            Console::default().render(&table)
        )
    }
}

fn sort_tuples<K: Ord, V>(tuples: impl IntoIterator<Item = (K, V)>) -> Vec<(K, V)> {
    let tuples = tuples.into_iter();
    let mut tuples = tuples.collect::<Vec<_>>();
    tuples.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    tuples
}
