pub trait SliceExt {
    fn sum(&self) -> f64;

    fn normalize(&mut self) -> f64;
}
impl SliceExt for [f64] {
    fn sum(&self) -> f64 {
        self.iter().sum()
    }

    fn normalize(&mut self) -> f64 {
        let sum = self.sum();
        for item in self {
            *item /= sum;
        }
        sum
    }
}

pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}
impl Fraction {
    pub fn quotient(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}