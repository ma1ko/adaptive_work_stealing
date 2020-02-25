#[macro_use]
extern crate lazy_static;
extern crate crossbeam_utils;
use crossbeam::CachePadded;
use crossbeam_utils as crossbeam;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};
// use rayon_logs as rayon;

pub const NUM_THREADS: usize = 4;
lazy_static! {
    static ref V: [CachePadded<AtomicUsize>; NUM_THREADS] = Default::default();
    // static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
}
pub fn steal(backoffs: usize, victim: usize) -> Option<()> {
    /*
    if x != 0 {
        return None;
    } // Only steal from 0 currently
    */

    V[victim].fetch_add(1, Ordering::Relaxed);
    let backoff = crossbeam::Backoff::new();
    let mut c: usize = 0;
    for _ in 0..backoffs {
        // wait until the victim has taken the value, check regularly
        backoff.snooze(); // or spin()?
        c = V[victim].load(Ordering::Relaxed);
        if c == 0 {
            return Some(());
        }
    }
    // let _ = V[victim].compare_exchange_weak(c, c - 1, Ordering::Relaxed, Ordering::Relaxed);

    None
}
pub fn main() -> Result<(), std::io::Error> {
    let mut numbers: Vec<usize> = (0..100_000).collect();
    let mut verify: Vec<usize> = numbers.clone();
    do_the_work(&mut verify);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .steal_callback(|x| steal(5, x))
        .build()
        .unwrap();

    pool.install(|| {
        run(&mut numbers);
    }); // currently syncronisation doesn't work , so we just wait a bit for everybody to finish
    assert_eq!(numbers, verify, "wrong output");
    Ok(())
}
const MIN_WORK_SIZE: usize = 100;
pub fn run(mut numbers: &mut [usize]) {
    let mut work_size = MIN_WORK_SIZE;
    // local copy
    let thread_index: usize = rayon::current_thread_index().unwrap();

    while !numbers.is_empty() {
        let len = numbers.len();
        // check how many other threads need work
        let mut steal_counter: usize = V[thread_index].swap(0, Ordering::Relaxed);
        if steal_counter > 0 && len >= MIN_WORK_SIZE {
            /*
            println!(
                "{}: {} steals, {} -> {}",
                thread_index, steal_counter, numbers[0], len
            );
            */
            if steal_counter > NUM_THREADS {
                // There's more steals than threads, just create tasks for all *other* threads
                steal_counter = NUM_THREADS - 1
            }
            let mut chunks =
                numbers.chunks_mut(len / (steal_counter + 1/* for me */) + 1 /*round up */);
            numbers = chunks.next().unwrap();
            fn spawn(chunks: &mut std::slice::ChunksMut<usize>) -> Option<()> {
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

        /*
        println!(
            "{} calculating {} to {}, {} elements",
            thread_index,
            left[0],
            left[left.len() - 1],
            left.len()
        );
        */

        do_the_work(left);
        numbers = right;
    }
}
pub fn work(x: &mut usize) {
    /* // We could also just spin for a while
    let backoff = crossbeam::Backoff::new();
    while !backoff.is_completed() {
        // simulate some work
        backoff.spin()
    }
    */
    // This just creates some static work
    // *x = (*x as f64).sqrt().sqrt().sqrt().sqrt() as usize;
    //
    // Try to find a good function to simulate work.
    // I'm choosing N! where N E [0,100], so work varies for each element.
    *x = (0..*x % 100).into_iter().fold(1, |x, y| (x * y));

    // Create an array and sum it up (complex version to stop the optimizer
    // *x = (0..*x / 1000).into_iter().collect::<Vec<usize>>()
    //.iter()
    // .sum();

    /* // maybe use a random number?
    use rand::Rng;
    let val = rand::thread_rng().gen_range(0, 100);
    *x = (0..val).collect::<Vec<usize>>().iter().sum();
    */
}

pub fn do_the_work(data: &mut [usize]) {
    data.iter_mut().for_each(|mut x: &mut usize| {
        work(&mut x);
    });
}
