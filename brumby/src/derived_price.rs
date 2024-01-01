use crate::market::MarketPrice;

#[derive(Debug, Clone, PartialEq)]
pub struct DerivedPrice {
    pub probability: f64,
    pub price: f64,
}
impl DerivedPrice {
    pub fn fair_price(&self) -> f64 {
        1.0 / self.probability
    }

    pub fn overround(&self) -> f64 {
        1.0 / self.probability / self.price
    }
}

impl Default for DerivedPrice {
    fn default() -> Self {
        Self {
            probability: 0.,
            price: f64::INFINITY
        }
    }
}

impl MarketPrice for DerivedPrice {
    fn decimal(&self) -> f64 {
        self.price
    }
}