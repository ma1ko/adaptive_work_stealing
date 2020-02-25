use adaptive_work_stealing::{do_the_work, run, steal, work, NUM_THREADS};
use criterion::Bencher;
use criterion::BenchmarkGroup;
use criterion::*;
use rayon::prelude::*;
#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref numbers: Vec<usize> = (0..WORK_SIZE).into_iter().collect();
    #[derive(Debug, Eq, PartialEq, Clone )]
    static ref VERIFY: Vec<usize> = numbers
        .clone()
        .iter_mut()
        .map(|x: &mut usize| {
            work(&mut *x);
            *x
        })
        .collect();
}

const WORK_SIZE: usize = 100_000;
pub fn adaptive(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    for size in [0, 2, 4, 6, 8].iter() {
        group.bench_with_input(BenchmarkId::new("Adaptive", size), &size, |b, &size| {
            //group.bench_function("adaptive", |b| {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(NUM_THREADS)
                .steal_callback(move |x| steal(*size, x))
                .build()
                .unwrap();

            b.iter_batched(
                || numbers.clone(),
                |mut n| {
                    pool.install(|| {
                        run(black_box(&mut n));
                    });
                    assert_eq!(n, *VERIFY, "wrong output");
                    n
                },
                BatchSize::SmallInput,
            );
        });
    }
}

pub fn iterator(b: &mut Bencher) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();

    b.iter_batched(
        || numbers.clone(),
        |mut n| {
            pool.install(|| {
                black_box(&mut n)
                    .par_iter_mut()
                    .for_each(|x: &mut usize| work(&mut *x));
            });
            assert_eq!(n, *VERIFY, "wrong output");
            n
        },
        BatchSize::SmallInput,
    );
}
pub fn single(b: &mut Bencher) {
    b.iter_batched(
        || numbers.clone(),
        |mut n| {
            black_box(&mut n)
                .iter_mut()
                .for_each(|x: &mut usize| work(&mut *x));
            assert_eq!(n, *VERIFY, "wrong output");
            n
        },
        BatchSize::SmallInput,
    );
}

pub fn perfect_split(b: &mut Bencher) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();
    pool.install(|| {
        b.iter_batched(
            || numbers.clone(),
            |mut n| {
                let (left, right) = black_box(&mut n).split_at_mut(WORK_SIZE / 2);
                rayon::join(
                    || {
                        let (mut left, mut right) = left.split_at_mut(WORK_SIZE / 4);
                        rayon::join(|| do_the_work(&mut left), || do_the_work(&mut right));
                    },
                    || {
                        let (mut left, mut right) = right.split_at_mut(WORK_SIZE / 4);
                        rayon::join(|| do_the_work(&mut left), || do_the_work(&mut right));
                    },
                );
                assert_eq!(n, *VERIFY, "wrong output");
                n
            },
            BatchSize::SmallInput,
        );
    });
}
fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Work-Stealing");
    group.measurement_time(std::time::Duration::new(5, 0));

    group.bench_function("single", |mut b: &mut Bencher| {
        single(&mut b);
    });

    group.bench_function("iterator", |mut b: &mut Bencher| {
        iterator(&mut b);
    });

    group.bench_function("perfect split", |b| {
        perfect_split(b);
    });
    adaptive(&mut group);

    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
