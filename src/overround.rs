const MIN_PRICE: f64 = 1.04;

pub fn apply_with_cap(fair_price: f64, overround: f64) -> f64 {
    f64::max(MIN_PRICE, fair_price / overround)
}