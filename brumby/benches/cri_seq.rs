use std::hash::{Hash, Hasher};
use std::mem;
use criterion::{criterion_group, criterion_main, Criterion, black_box};
use brumby::stack_vec::raw_array::RawArray;
use brumby::stack_vec::StackVec;

struct CountingHasher(u64);
impl Hasher for CountingHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0 += bytes.len() as u64;
    }
}

#[inline(always)]
fn compute_hash(seq: &impl Hash) -> u64 {
    let mut hasher = CountingHasher(0);
    seq.hash(&mut hasher);
    hasher.finish()
}

#[inline(always)]
fn create_populated_vec<const C: usize>() -> Vec<usize> {
    let mut vec = Vec::<usize>::with_capacity(C);
    black_box(&vec);
    (0..C).for_each(|i| vec.push(i));
    vec
}

#[inline(always)]
fn create_populated_sv<const C: usize>() -> StackVec<usize, C> {
    let mut sv = StackVec::default();
    black_box(&sv);
    (0..C).for_each(|i| sv.push(i));
    sv
}

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
            vec.len()
        }

        #[inline(always)]
        fn hash<const C: usize>() -> u64 {
            let vec = create_populated_vec::<C>();
            compute_hash(&vec)
        }

        #[inline(always)]
        fn eq<const C: usize>() -> bool {
            let a = create_populated_vec::<C>();
            let b = create_populated_vec::<C>();
            a.eq(&b)
        }

        // sanity checks
        assert_eq!(10, sum::<5>());
        assert_eq!(0, create::<5>());
        assert_eq!((6 * mem::size_of::<usize>()) as u64, hash::<5>());
        assert!(eq::<5>());

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

        fn bench_hash<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_hash_vec_{C}"), |b| {
                b.iter(|| hash::<C>());
            });
        }
        bench_hash::<16>(c);
        bench_hash::<64>(c);
        bench_hash::<256>(c);
        bench_hash::<1024>(c);
        bench_hash::<4096>(c);

        fn bench_eq<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_eq_vec_{C}"), |b| {
                b.iter(|| eq::<C>());
            });
        }
        bench_eq::<16>(c);
        bench_eq::<64>(c);
        bench_eq::<256>(c);
        bench_eq::<1024>(c);
        bench_eq::<4096>(c);
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

        // sanity checks
        assert_eq!(10, sum::<5>());
        assert_eq!(5, create::<5>());

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

        #[inline(always)]
        fn hash<const C: usize>() -> u64 {
            let sv = create_populated_sv::<C>();
            compute_hash(&sv)
        }

        #[inline(always)]
        fn eq<const C: usize>() -> bool {
            let a = create_populated_sv::<C>();
            let b = create_populated_sv::<C>();
            a.eq(&b)
        }

        // sanity checks
        assert_eq!(10, sum::<5>());
        assert_eq!(0, create::<5>());
        assert_eq!((6 * mem::size_of::<usize>()) as u64, hash::<5>());
        assert!(eq::<5>());

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

        fn bench_hash<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_hash_sv_{C}"), |b| {
                b.iter(|| hash::<C>());
            });
        }
        bench_hash::<16>(c);
        bench_hash::<64>(c);
        bench_hash::<256>(c);
        bench_hash::<1024>(c);
        bench_hash::<4096>(c);

        fn bench_eq<const C: usize>(c: &mut Criterion) {
            c.bench_function(&format!("cri_seq_eq_sv_{C}"), |b| {
                b.iter(|| eq::<C>());
            });
        }
        bench_eq::<16>(c);
        bench_eq::<64>(c);
        bench_eq::<256>(c);
        bench_eq::<1024>(c);
        bench_eq::<4096>(c);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
