use criterion::{criterion_group, criterion_main, Criterion};
use brumby::stack_vec::raw_array::RawArray;
use brumby::stack_vec::StackVec;

fn criterion_benchmark(c: &mut Criterion) {
    {
        #[inline]
        fn test<const C: usize>() -> usize {
            let mut vec = Vec::with_capacity(C);
            for i in 0..C {
                vec.push(i);
            }
            let mut sum = 0;
            for i in 0..C {
                sum += vec[i];
            }
            sum
        }

        // sanity check
        assert_eq!(10, test::<5>());
        fn bench<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_vec_{C}"), |b| {
                b.iter(|| test::<C>());
            });
        }
        bench::<4>(c);
        bench::<16>(c);
        bench::<64>(c);
        bench::<256>(c);
        bench::<1024>(c);
        bench::<4096>(c);
    }
    {
        #[inline]
        fn test<const C: usize>() -> usize {
            let mut array = [0; C];
            for i in 0..C {
                array[i] = i;
            }
            let mut sum = 0;
            for i in 0..C {
                sum += array[i];
            }
            sum
        }

        // sanity check
        assert_eq!(10, test::<5>());
        fn bench<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_array_{C}"), |b| {
                b.iter(|| test::<C>());
            });
        }
        bench::<4>(c);
        bench::<16>(c);
        bench::<64>(c);
        bench::<256>(c);
        bench::<1024>(c);
        bench::<4096>(c);
    }
    {
        #[inline]
        fn test<const C: usize>() -> usize {
            let mut array = RawArray::<usize, C>::default();
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
        assert_eq!(10, test::<5>());
        fn bench<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_raw_{C}"), |b| {
                b.iter(|| test::<C>());
            });
        }
        bench::<4>(c);
        bench::<16>(c);
        bench::<64>(c);
        bench::<256>(c);
        bench::<1024>(c);
        bench::<4096>(c);
    }
    {
        #[inline]
        fn test<const C: usize>() -> usize {
            let mut sv = StackVec::<usize, C>::default();
            for i in 0..C {
                sv.push(i);
            }
            let mut sum = 0;
            for i in 0..C {
                sum += sv[i];
            }
            sum
        }

        // sanity check
        assert_eq!(10, test::<5>());
        fn bench<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_sv_{C}"), |b| {
                b.iter(|| test::<C>());
            });
        }
        bench::<4>(c);
        bench::<16>(c);
        bench::<64>(c);
        bench::<256>(c);
        bench::<1024>(c);
        bench::<4096>(c);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
