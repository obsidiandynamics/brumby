use criterion::{criterion_group, criterion_main, Criterion, black_box};
use brumby::stack_vec::raw_array::RawArray;
use brumby::stack_vec::StackVec;

fn criterion_benchmark(c: &mut Criterion) {
    {
        #[inline(always)]
        fn sum<const C: usize>() -> usize {
            let mut vec = Vec::with_capacity(C);
            black_box(&vec);
            for i in 0..C {
                vec.push(i);
            }
            let mut sum = 0;
            for i in 0..C {
                sum += vec[i];
            }
            sum
        }

        #[inline(always)]
        fn create<const C: usize>() -> usize {
            let vec = Vec::<usize>::with_capacity(C);
            black_box(&vec);
            vec[0] + vec.len()
        }

        // sanity check
        assert_eq!(10, sum::<5>());
        fn bench_sum<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_sum_vec_{C}"), |b| {
                b.iter(|| sum::<C>());
            });
        }
        bench_sum::<16>(c);
        bench_sum::<64>(c);
        bench_sum::<256>(c);
        bench_sum::<1024>(c);
        bench_sum::<4096>(c);

        fn bench_create<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_create_vec_{C}"), |b| {
                b.iter(|| create::<C>());
            });
        }
        bench_create::<16>(c);
        bench_create::<64>(c);
        bench_create::<256>(c);
        bench_create::<1024>(c);
    }
    {
        #[inline(always)]
        fn sum<const C: usize>() -> usize {
            let mut array = [0; C];
            black_box(&array);
            for i in 0..C {
                array[i] = i;
            }
            let mut sum = 0;
            for i in 0..C {
                sum += array[i];
            }
            sum
        }

        #[inline(always)]
        fn create<const C: usize>() -> usize {
            let array = [0; C];
            black_box(&array);
            array.len()
        }

        // sanity check
        assert_eq!(10, sum::<5>());
        fn bench_sum<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_sum_array_{C}"), |b| {
                b.iter(|| sum::<C>());
            });
        }
        bench_sum::<16>(c);
        bench_sum::<64>(c);
        bench_sum::<256>(c);
        bench_sum::<1024>(c);
        bench_sum::<4096>(c);

        fn bench_create<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_create_array_{C}"), |b| {
                b.iter(|| create::<C>());
            });
        }
        bench_create::<16>(c);
        bench_create::<64>(c);
        bench_create::<256>(c);
        bench_create::<1024>(c);
    }
    {
        #[inline(always)]
        fn sum<const C: usize>() -> usize {
            let mut array = RawArray::<usize, C>::default();
            black_box(&array);
            unsafe {
                for i in 0..C {
                    array.set_and_forget(i, i);
                }
                let mut sum = 0;
                for i in 0..C {
                    sum += array.get(i);
                }
                array.destructor(0, C);
                sum
            }
        }

        // sanity check
        assert_eq!(10, sum::<5>());
        fn bench_sum<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_sum_raw_{C}"), |b| {
                b.iter(|| sum::<C>());
            });
        }
        bench_sum::<16>(c);
        bench_sum::<64>(c);
        bench_sum::<256>(c);
        bench_sum::<1024>(c);
        bench_sum::<4096>(c);
    }
    {
        #[inline(always)]
        fn sum<const C: usize>() -> usize {
            let mut sv = StackVec::<usize, C>::default();
            black_box(&sv);
            for i in 0..C {
                sv.push(i);
            }
            let mut sum = 0;
            for i in 0..C {
                sum += sv[i];
            }
            sum
        }

        #[inline(always)]
        fn create<const C: usize>() -> usize {
            let sv = StackVec::<usize, C>::default();
            black_box(&sv);
            sv.len()
        }

        // sanity check
        assert_eq!(10, sum::<5>());
        fn bench_sum<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_sum_sv_{C}"), |b| {
                b.iter(|| sum::<C>());
            });
        }
        bench_sum::<16>(c);
        bench_sum::<64>(c);
        bench_sum::<256>(c);
        bench_sum::<1024>(c);
        bench_sum::<4096>(c);

        fn bench_create<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_create_sv_{C}"), |b| {
                b.iter(|| create::<C>());
            });
        }
        bench_create::<16>(c);
        bench_create::<64>(c);
        bench_create::<256>(c);
        bench_create::<1024>(c);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
