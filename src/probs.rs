pub trait VecExt {
    fn sum(&self) -> f64;

    fn normalize(&mut self) -> f64;
}
impl VecExt for Vec<f64> {
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