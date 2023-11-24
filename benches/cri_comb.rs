use criterion::{criterion_group, criterion_main, Criterion};

use brumby::comb::{count_combinations, is_unique_linear, pick};

fn criterion_benchmark(c: &mut Criterion) {
    fn fixtures(items: usize, times: usize) -> (Vec<bool>, Vec<usize>, Vec<usize>) {
        let mut bitmap = Vec::with_capacity(items);
        bitmap.resize(bitmap.capacity(), true);
        let mut cardinalities = Vec::with_capacity(times);
        cardinalities.resize(cardinalities.capacity(), items);
        let mut ordinals = Vec::with_capacity(times);
        ordinals.resize(ordinals.capacity(), 0usize);
        (bitmap, cardinalities, ordinals)
    }

    let (mut bitmap, cardinalities, mut ordinals) = fixtures(10, 5);

    // sanity check
    let unique_combinations = (0..count_combinations(&cardinalities))
        .into_iter()
        .map(|combination| {
            pick(&cardinalities, combination, &mut ordinals);
            is_unique_linear(&ordinals, &mut bitmap)
        })
        .filter(|&unique| unique)
        .count();
    assert_eq!(10 * 9 * 8 * 7 * 6, unique_combinations);

    fn bench(c: &mut Criterion, items: usize, times: usize) {
        let (mut bitmap, cardinalities, mut ordinals) = fixtures(items, times);
        c.bench_function(&format!("cri_comb_{items}c{times}"), |b| {
            b.iter(|| {
                for combination in 0..count_combinations(&cardinalities) {
                    pick(&cardinalities, combination, &mut ordinals);
                    is_unique_linear(&ordinals, &mut bitmap);
                }
            });
        });
    }
    bench(c, 10, 3);
    bench(c, 10, 4);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
