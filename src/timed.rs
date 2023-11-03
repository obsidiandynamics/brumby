//! Timing of computations.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct Timed<V> {
    pub value: V,
    pub elapsed: Duration,
}
impl<V> Timed<V> {
    pub fn result<E>(f: impl FnOnce() -> Result<V, E>) -> Result<Timed<V>, E> {
        let start_time = Instant::now();
        f().map(|value| {
            let elapsed = start_time.elapsed();
            Timed {
                value,
                elapsed
            }
        })
    }
}