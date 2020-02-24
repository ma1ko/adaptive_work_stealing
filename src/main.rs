#[macro_use]
extern crate lazy_static;
extern crate crossbeam_utils;
use crossbeam_utils as crossbeam;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};
// use rayon_logs as rayon;

const NUM_THREADS: usize = 4;
lazy_static! {
    static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
}
lazy_static! {}
fn main() -> Result<(), std::io::Error> {
    let mut numbers: Vec<u64> = (0..100000).into_iter().collect();
    let verify: Vec<u64> = numbers.iter().map(|x| x * 2).collect();
    fn steal(victim: usize) -> Option<()> {
        /*
        if x != 0 {
            return None;
        } // Only steal from 0 currently
        */

        V[victim].fetch_add(1, Ordering::Relaxed);
        let backoff = crossbeam::Backoff::new();
        let mut c = 0;
        for _ in 0..5 {
            // wait until the victim has taken the value, check regularly
            backoff.snooze(); // or spin()?
            c = V[victim].load(Ordering::Relaxed);
            if c == 0 {
                return Some(());
            }
        }
        let prev = V[victim].compare_and_swap(c, c - 1, Ordering::Relaxed);
        /* // Do we retry decreasing the counter or not?
        if prev != c {
            println!("Failed decreasing {}", victim);
        }
        */

        None
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .steal_callback(steal)
        .build()
        .unwrap();

    pool.install(|| {
        run(&mut numbers);
    }); // currently syncronisation doesn't work , so we just wait a bit for everybody to finish
    assert_eq!(numbers, verify, "wrong output");
    Ok(())
}
const MIN_WORK_SIZE: usize = 100;
fn run(mut numbers: &mut [u64]) {
    let mut work_size = MIN_WORK_SIZE;
    // local copy
    let mut numbers: &mut [u64] = &mut numbers;
    let thread_index: usize = rayon::current_thread_index().unwrap();

    while numbers.len() > 0 {
        let len = numbers.len();
        // check how many other threads need work
        let mut steal_counter: usize = V[thread_index].swap(0, Ordering::Relaxed);
        if steal_counter > 0 && len >= MIN_WORK_SIZE {
            println!(
                "{}: {} steals, {} -> {}",
                thread_index, steal_counter, numbers[0], len
            );
            if steal_counter > NUM_THREADS {
                // There's more steals than threads, just create tasks for all *other* threads
                steal_counter = NUM_THREADS - 1
            }
            let mut chunks = numbers.chunks_mut(len / (steal_counter + 1/* for me */) + 1);
            numbers = chunks.next().unwrap();
            fn spawn(chunks: &mut std::slice::ChunksMut<u64>) -> Option<()> {
                let chunk = chunks.next()?;
                rayon::join(
                    || {
                        spawn(chunks);
                    },
                    || {
                        run(chunk);
                    },
                );
                None
            };
            spawn(&mut chunks);
            work_size = MIN_WORK_SIZE;
        }
        // do *some* work, here: 100 elements
        let (left, right) = numbers.split_at_mut(std::cmp::min(numbers.len(), work_size));
        work_size *= 2;

        println!(
            "{} calculating {} to {}, {} elements",
            thread_index,
            left[0],
            left[left.len() - 1],
            left.len()
        );

        do_the_work(left);
        numbers = right;
    }
}

fn do_the_work(data: &mut [u64]) {
    data.iter_mut().for_each(|x: &mut u64| {
        let backoff = crossbeam::Backoff::new();
        while !backoff.is_completed() {
            // simulate some work
            backoff.snooze()
        }
        *x = 2 * *x
    });
}
