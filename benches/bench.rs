use adaptive_work_stealing::{do_the_work, run, steal, work, NUM_THREADS};
use criterion::Bencher;
use criterion::BenchmarkGroup;
use criterion::*;
use rayon_logs::prelude::*;
#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref NUMBERS: Vec<usize> = (0..WORK_SIZE).into_iter().collect();
    #[derive(Debug, Eq, PartialEq, Clone )]
    static ref VERIFY: usize = do_the_work(&NUMBERS).into_iter().sum();

}

fn verify(numbers: Vec<&usize>) {
    assert_eq!(numbers.into_iter().sum::<usize>(), *VERIFY, "wrong output");
}

const WORK_SIZE: usize = 50_000;
pub fn adaptive(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    let mut tasks: Vec<(usize, f64)> = Vec::new();
    for size in 1..5 {
        let mut num_tasks = (0, 0);
        group.bench_with_input(BenchmarkId::new("Adaptive", size), &size, |b, &size| {
            let pool = rayon_logs::ThreadPoolBuilder::new()
                .num_threads(size)
                .steal_callback(move |x| steal(8, x))
                .build()
                .unwrap();

            b.iter(|| {
                let (_, log) = pool.logging_install(|| {
                    let numbers = run(black_box(&NUMBERS));
                    verify(numbers);
                });
                num_tasks.0 += log.tasks_logs.len();
                num_tasks.1 += 1;
            });
        });
        let avg_tasks = num_tasks.0 as f64 / num_tasks.1 as f64;
        println!("{} tasks", avg_tasks);
        tasks.push((size, avg_tasks));
    }
    println!("{:?}", tasks);
}

pub fn iterator(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    let mut tasks: Vec<(usize, f64)> = Vec::new();
    for size in 1..5 {
        let mut num_tasks = (0, 0);
        group.bench_with_input(BenchmarkId::new("Iterator", size), &size, |b, &size| {
            let pool = rayon_logs::ThreadPoolBuilder::new()
                .num_threads(size)
                .build()
                .unwrap();

            b.iter(|| {
                let (_, log) = pool.logging_install(|| {
                    let numbers: Vec<&usize> = black_box(&NUMBERS)
                        .par_iter()
                        .filter(|x| work(**x))
                        .collect();
                    verify(numbers);
                });
                num_tasks.0 += log.tasks_logs.len();
                num_tasks.1 += 1;
            });
        });
        let avg_tasks = num_tasks.0 as f64 / num_tasks.1 as f64;
        println!("{} tasks", avg_tasks);
        tasks.push((size, avg_tasks));
    }
    println!("{:?}", tasks);
}
pub fn single(b: &mut Bencher) {
    b.iter(|| {
        let numbers = do_the_work(black_box(&NUMBERS));
        verify(numbers);
    });
}

pub fn perfect_split(b: &mut Bencher) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();
    fn rec_split(levels: usize, data: &[usize]) -> Vec<&usize> {
        if levels == 0 {
            return do_the_work(data);
        }

        let (left, right) = data.split_at(data.len() / 2);
        let (mut left, mut right) = rayon::join(
            || rec_split(levels - 1, left),
            || rec_split(levels - 1, right),
        );
        left.append(&mut right);
        left
    }

    b.iter(|| {
        pool.install(|| {
            let result = rec_split(
                (NUM_THREADS as f64).log2().ceil() as usize,
                black_box(&NUMBERS),
            );
            verify(result);
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
    /*
    group.bench_function("iterator", |mut b: &mut Bencher| {
        iterator(&mut b);
    });
    */
    adaptive(&mut group);
    iterator(&mut group);

    /*
    group.bench_function("perfect split", |b| {
        perfect_split(b);
    });
    */

    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
