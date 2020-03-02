#[macro_use]
extern crate lazy_static;
extern crate crossbeam_utils;
use crossbeam::CachePadded;
use crossbeam_utils as crossbeam;
// use rayon_logs as rayon;
use rayon::prelude::*;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const NUM_THREADS: usize = 4;
lazy_static! {
    static ref V: [CachePadded<AtomicUsize>; NUM_THREADS] = Default::default();
    // static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
}
pub fn steal(backoffs: usize, victim: usize) -> Option<()> {
    //rayon_logs::subgraph("WAITING", backoffs, move || {
    let thread_index = rayon::current_thread_index().unwrap();
    let thread_index = 1 << thread_index;
    V[victim].fetch_or(thread_index, Ordering::Relaxed);

    let backoff = crossbeam::Backoff::new();
    let mut c: usize = 0;
    for _ in 0..backoffs {
        backoff.snooze(); // or spin()?

        // wait until the victim has taken the value, check regularly
        c = V[victim].load(Ordering::Relaxed);
        if c & thread_index == 0 {
            return Some(());
        }
    }

    //V[victim].fetch_and(!thread_index, Ordering::Relaxed);
    /*
     let _ = V[victim].compare_exchange_weak(
         c,
         c & !(thread_index),
         Ordering::Relaxed,
         Ordering::Relaxed,
     );
    */

    None
}
const N: usize = 20_000;
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let numbers: Vec<usize> = (0..N).collect();
    let mut verify: Vec<usize> = do_the_work(&numbers);
    do_the_work(&mut verify);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .steal_callback(|x| steal(4, x))
        .build()?;
    //let (_, log) = pool.logging_install(|| {
    pool.install(|| {
        let results = run(&numbers);
        assert_eq!(*results, verify, "wrong output");
    });
    //});

    //log.save_svg("test.svg").expect("failed saving svg");
    //log.save("test").expect("failed saving log");
    Ok(())
}
const MIN_WORK_SIZE: usize = 10;
pub fn run(mut numbers: &[usize]) -> Box<Vec<usize>> {
    let mut filtered: Box<Vec<usize>> = Box::new(Vec::new());
    let mut work_size = MIN_WORK_SIZE;
    while !numbers.is_empty() {
        let len = numbers.len();
        // check how many other threads need work
        let thread_index = rayon::current_thread_index().unwrap();
        let steal_counter = V[thread_index].swap(0, Ordering::SeqCst);
        let steal_counter = steal_counter.count_ones() as usize;
        if steal_counter > 0 && len >= MIN_WORK_SIZE {
            // If there's more steals than threads, just create tasks for all *other* threads
            let steal_counter = std::cmp::min(steal_counter, NUM_THREADS - 1);
            let chunks = numbers
                .chunks(len / (steal_counter + 1/* for me */) + 1 /*round up */)
                .peekable();
            fn spawn(
                mut chunks: std::iter::Peekable<std::slice::Chunks<usize>>,
            ) -> Box<Vec<usize>> {
                let chunk = chunks.next().unwrap();
                match chunks.peek() {
                    None => {
                        // finished recursion, let's do our part of the data
                        return run(chunk);
                    }
                    Some(_) => {
                        let (mut left, mut right) = rayon::join(
                            || {
                                // prepare another task for the next stealer
                                spawn(chunks)
                            },
                            || {
                                // let the stealer process it's part
                                run(chunk)
                            },
                        );
                        //   rayon_logs::subgraph("merge", left.len(), || {
                        right.append(&mut left);
                        //   });
                        right
                    }
                }
            }
            let mut res: Box<Vec<usize>> = spawn(chunks);
            filtered.append(&mut res);
            return filtered;
        // break; // we are finished processing, return the recursion
        } else {
            // do *some* work, here: we start with work_size and double every round
            let (left, right) = numbers.split_at(std::cmp::min(numbers.len(), work_size));
            //rayon_logs::subgraph("Work", work_size, || {
            let mut res = do_the_work(left);
            filtered.append(&mut res);
            //});
            // Nobody stole, so we might do more work next time
            // maximim is either everything that's left or sqrt(N), so we don't let other threads
            // wait too long
            work_size = std::cmp::min(work_size * 2, (N as f64).sqrt() as usize);
            numbers = right;
        }
    }
    filtered
}
pub fn work(x: usize) -> bool {
    /* // We could also just spin for a while
    let backoff = crossbeam::Backoff::new();
    while !backoff.is_completed() {
        // simulate some work
        backoff.spin()
    }
    */
    // This just creates some static work
    //*x = (*x as f64).sqrt().sqrt().sqrt().sqrt() as usize;
    //
    // Try to find a good function to simulate work.
    // I'm choosing N! where N E [0,50], so work varies for each element.
    //*x = (1..*x % 50).into_iter().fold(1, |x, y| (x * y) % 1_000_000);
    //*x = *x * 2;
    is_prime(x)

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
fn is_prime(n: usize) -> bool {
    for a in 2..(n as f64).sqrt() as usize {
        if n % a == 0 {
            return false; // if it is not the last statement you need to use `return`
        }
    }
    true // last value to return
}

pub fn do_the_work(data: &[usize]) -> Vec<usize> {
    data.iter().filter(|x| work(**x)).cloned().collect()
}
