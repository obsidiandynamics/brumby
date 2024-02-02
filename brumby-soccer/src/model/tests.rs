use rustc_hash::FxHashMap;
use stanza::renderer::console::Console;
use stanza::renderer::Renderer;
use crate::domain::{DrawHandicap, Offer, OfferType, Outcome, Period, Side, WinHandicap};
use crate::model::{Config, Model, Stub};
use brumby::hash_lookup::HashLookup;
use brumby::market::{Market, Overround, OverroundMethod, PriceBounds};
use crate::print;

const SINGLE_PRICE_BOUNDS: PriceBounds = 1.001..=1001.0;
const OVERROUND: Overround = Overround {
    method: OverroundMethod::Multiplicative,
    value: 1.0,
};
const EPSILON: f64 = 1e06;

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
        market: Market::fit(
            &OVERROUND.method,
            fair_prices,
            1.0,
        ),
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
        market: Market::fit(
            &OverroundMethod::Multiplicative,
            fair_prices,
            1.0,
        ),
    });
}

fn stub_split_handicap(draw_handicap: DrawHandicap, win_handicap: WinHandicap) -> Stub {
    let outcomes = HashLookup::from([
        Outcome::SplitWin(Side::Home, draw_handicap.clone(), win_handicap.clone()),
        Outcome::SplitWin(Side::Away, draw_handicap.flip(), win_handicap.flip_asian())
    ]);
    Stub {
        offer_type: OfferType::SplitHandicap(Period::FullTime, draw_handicap, win_handicap),
        outcomes,
        normal: 1.0,
        overround: OVERROUND.clone(),
    }
}

#[test]
pub fn split_handicap_evenly_matched() {
    let mut model = create_test_model();
    insert_head_to_head(&mut model, DrawHandicap::Ahead(2), vec![12.26, 8.2, 1.25]);
    insert_head_to_head(&mut model, DrawHandicap::Ahead(1), vec![4.91, 4.88, 1.68]);
    insert_head_to_head(&mut model, DrawHandicap::Ahead(0), vec![2.44, 4.13, 2.85]);
    insert_head_to_head(&mut model, DrawHandicap::Behind(1), vec![1.53, 5.33, 6.15]);
    insert_asian_handicap(&mut model, WinHandicap::AheadOver(0), vec![2.44, 1.68]);
    insert_asian_handicap(&mut model, WinHandicap::BehindUnder(1), vec![1.53, 2.85]);

    model.derive(&[
        stub_split_handicap(DrawHandicap::Ahead(1), WinHandicap::AheadOver(0)),
        stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::AheadOver(0)),
        stub_split_handicap(DrawHandicap::Ahead(0), WinHandicap::BehindUnder(1)),
        stub_split_handicap(DrawHandicap::Behind(1), WinHandicap::BehindUnder(1)),
    ], &SINGLE_PRICE_BOUNDS).unwrap();

    let offer = model.offers().get(&OfferType::SplitHandicap(Period::FullTime, DrawHandicap::Ahead(1), WinHandicap::AheadOver(0))).unwrap();
    // assert_slice_f64_relative(&[2.156, 1.865], &offer.market.prices, EPSILON);

    print_offers(model.offers());
}

fn print_offers(offers: &FxHashMap<OfferType, Offer>) {
    for (_, offer) in sort_tuples(offers) {
        let table = print::tabulate_offer(offer);
        println!("{:?}:\n{}", offer.offer_type, Console::default().render(&table))
    }
}

fn sort_tuples<K: Ord, V>(tuples: impl IntoIterator<Item = (K, V)>) -> Vec<(K, V)> {
    let tuples = tuples.into_iter();
    let mut tuples = tuples.collect::<Vec<_>>();
    tuples.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    tuples
}