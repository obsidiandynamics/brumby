use assert_float_eq::*;
use criterion::{criterion_group, criterion_main, Criterion};
use std::ops::RangeInclusive;

use brumby::capture::Capture;
use brumby::opt::{hypergrid_search, HypergridSearchConfig, HypergridSearchOutcome};

fn criterion_benchmark(c: &mut Criterion) {
    const RANGE: RangeInclusive<f64> = -10.0..=10.0;

    #[inline(always)]
    fn poly3() -> HypergridSearchOutcome<3> {
        let config = HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-12,
            bounds: Capture::Borrowed(&[RANGE; 3]),
            resolution: 4,
        };
        // search for root of (x - 5)(x + 6)(x - 10) = 0
        hypergrid_search(
            &config,
            |_| true,
            |values| {
                (values[0] - 5.0).powi(2) + (values[1] + 6.0).powi(2) + (values[2] - 10.0).powi(2)
            },
        )
    }

    #[inline(always)]
    fn poly4() -> HypergridSearchOutcome<4> {
        let config = HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-15,
            bounds: Capture::Borrowed(&[RANGE; 4]),
            resolution: 4,
        };
        // search for root of (x - 5)(x + 6)(x - 10)(x + 3) = 0
        hypergrid_search(
            &config,
            |_| true,
            |values| {
                (values[0] - 5.0).powi(2)
                    + (values[1] + 6.0).powi(2)
                    + (values[2] - 10.0).powi(2)
                    + (values[3] + 3.0).powi(2)
            },
        )
    }

    #[inline(always)]
    fn poly5() -> HypergridSearchOutcome<5> {
        let config = HypergridSearchConfig {
            max_steps: 100,
            acceptable_residual: 1e-15,
            bounds: Capture::Borrowed(&[RANGE; 5]),
            resolution: 4,
        };
        // search for root of (x - 5)(x + 6)(x - 10)(x + 3)(x + 9) = 0
        hypergrid_search(
            &config,
            |_| true,
            |values| {
                (values[0] - 5.0).powi(2)
                    + (values[1] + 6.0).powi(2)
                    + (values[2] - 10.0).powi(2)
                    + (values[3] + 3.0).powi(2)
                    + (values[4] + 9.0).powi(2)
            },
        )
    }

    // sanity check
    {
        let outcome = poly3();
        assert_float_absolute_eq!(5.0, outcome.optimal_values[0]);
        assert_float_absolute_eq!(-6.0, outcome.optimal_values[1]);
        assert_float_absolute_eq!(10.0, outcome.optimal_values[2]);
    }
    {
        let outcome = poly4();
        assert_float_absolute_eq!(5.0, outcome.optimal_values[0]);
        assert_float_absolute_eq!(-6.0, outcome.optimal_values[1]);
        assert_float_absolute_eq!(10.0, outcome.optimal_values[2]);
        assert_float_absolute_eq!(-3.0, outcome.optimal_values[3]);
    }
    {
        let outcome = poly5();
        assert_float_absolute_eq!(5.0, outcome.optimal_values[0]);
        assert_float_absolute_eq!(-6.0, outcome.optimal_values[1]);
        assert_float_absolute_eq!(10.0, outcome.optimal_values[2]);
        assert_float_absolute_eq!(-3.0, outcome.optimal_values[3]);
        assert_float_absolute_eq!(-9.0, outcome.optimal_values[4]);
    }

    c.bench_function(&format!("cri_hypergrid_poly3"), |b| {
        b.iter(|| poly3());
    });
    c.bench_function(&format!("cri_hypergrid_poly4"), |b| {
        b.iter(|| poly4());
    });
    c.bench_function(&format!("cri_hypergrid_poly5"), |b| {
        b.iter(|| poly5());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
