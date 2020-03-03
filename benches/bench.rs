use adaptive_work_stealing::{do_the_work, run, steal, work, NUM_THREADS};
use criterion::Bencher;
use criterion::BenchmarkGroup;
use criterion::*;
use rayon::prelude::*;
#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref NUMBERS: Vec<usize> = (0..WORK_SIZE).into_iter().collect();
    #[derive(Debug, Eq, PartialEq, Clone )]
    static ref VERIFY: Vec<usize> = do_the_work(&NUMBERS);

}

const WORK_SIZE: usize = 10_000;
pub fn adaptive(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    for size in [5, 10, 15, 20, 25, 30, 35, 40, 45, 50].iter() {
        group.bench_with_input(BenchmarkId::new("Adaptive", size), &size, |b, &size| {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(NUM_THREADS)
                .steal_callback(move |x| steal(*size, x))
                .build()
                .unwrap();

            b.iter(|| {
                pool.install(|| {
                    let numbers = run(black_box(&NUMBERS));
                    assert_eq!(*numbers, *VERIFY, "wrong output");
                    numbers
                })
            });
        });
    }
}

pub fn iterator(b: &mut Bencher) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();

    b.iter(|| {
        pool.install(|| {
            let numbers: Vec<usize> = NUMBERS.par_iter().filter(|x| work(**x)).cloned().collect();
            assert_eq!(numbers, *VERIFY, "wrong output");
            numbers
        });
    });
}
pub fn single(b: &mut Bencher) {
    b.iter(|| {
        let numbers = do_the_work(black_box(&NUMBERS));
        assert_eq!(numbers, *VERIFY, "wrong output");
        numbers
    });
}

pub fn perfect_split(b: &mut Bencher) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();
    pool.install(|| {
        b.iter(|| {
            let (left, right) = black_box(&NUMBERS).split_at(WORK_SIZE / 2);
            let (mut left, mut right) = rayon::join(
                || {
                    let (mut left, mut right) = left.split_at(WORK_SIZE / 4);
                    let (mut left, mut right) =
                        rayon::join(|| do_the_work(&mut left), || do_the_work(&mut right));
                    left.append(&mut right);
                    left
                },
                || {
                    let (mut left, mut right) = right.split_at(WORK_SIZE / 4);
                    let (mut left, mut right) =
                        rayon::join(|| do_the_work(&mut left), || do_the_work(&mut right));
                    left.append(&mut right);
                    left
                },
            );
            left.append(&mut right);

            assert_eq!(left, *VERIFY, "wrong output");
            left
        });
    });
}
fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Work-Stealing");
    group.measurement_time(std::time::Duration::new(3, 0));
    group.warm_up_time(std::time::Duration::new(1, 0));
    group.sample_size(10);

    /*
    group.bench_function("single", |mut b: &mut Bencher| {
        single(&mut b);
    });
    */
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
