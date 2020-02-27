#[macro_use]
extern crate lazy_static;
extern crate crossbeam_utils;
use crossbeam::CachePadded;
use crossbeam_utils as crossbeam;
use rayon_logs as rayon;
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
    //V[victim].fetch_add(1, Ordering::Relaxed);
    V[victim].fetch_or(1 << thread_index, Ordering::Relaxed);

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
    //   None
    //});
    None
}
const N: usize = 500000;
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut numbers: Vec<usize> = (0..N).collect();
    let mut verify: Vec<usize> = numbers.clone();
    do_the_work(&mut verify);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .steal_callback(|x| steal(5, x))
        .build()?;
    let (_, log) = pool.logging_install(|| {
        run(&mut numbers);
    });

    log.save_svg("test.svg").expect("failed saving svg");
    log.save("test").expect("failed saving log");
    assert_eq!(numbers, verify, "wrong output");
    Ok(())
}
const MIN_WORK_SIZE: usize = 100;
pub fn run(mut numbers: &mut [usize]) {
    let mut work_size = MIN_WORK_SIZE;
    while !numbers.is_empty() {
        let len = numbers.len();
        // check how many other threads need work
        let thread_index = rayon::current_thread_index().unwrap();
        let steal_counter = V[thread_index].swap(0, Ordering::Relaxed);
        let mut steal_counter: usize = steal_counter.count_ones() as usize;
        if steal_counter > 0 && len >= MIN_WORK_SIZE {
            // If there's more steals than threads, just create tasks for all *other* threads
            steal_counter = std::cmp::min(steal_counter, NUM_THREADS - 1);
            let mut chunks = numbers
                .chunks_mut(len / (steal_counter + 1/* for me */) + 1 /*round up */)
                .peekable();
            fn spawn(chunks: &mut std::iter::Peekable<std::slice::ChunksMut<usize>>) {
                let chunk = chunks.next().unwrap();
                match chunks.peek() {
                    None => {
                        // finished recursion, let's do our part of the data
                        run(chunk);
                    }
                    Some(_) => {
                        rayon::join(
                            || {
                                // prepare another task for the next stealer
                                spawn(chunks);
                            },
                            || {
                                // let the stealer process it's part
                                run(chunk);
                            },
                        );
                    }
                }
            }
            spawn(&mut chunks);
            break; // we are finished processing, return the recursion
        } else {
            // do *some* work, here: we start with work_size and double every round
            let (left, right) = numbers.split_at_mut(std::cmp::min(numbers.len(), work_size));
            rayon_logs::subgraph("Work", work_size, || {
                do_the_work(left);
            });
            // Nobody stole, so we might do more work next time
            // maximim is either everything that's left or sqrt(N), so we don't let other threads
            // wait too long
            work_size = std::cmp::min(work_size * 2, (N as f64).sqrt() as usize);
            numbers = right;
        }
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
    //*x = (*x as f64).sqrt().sqrt().sqrt().sqrt() as usize;
    //
    // Try to find a good function to simulate work.
    // I'm choosing N! where N E [0,50], so work varies for each element.
    *x = (1..*x % 50).into_iter().fold(1, |x, y| (x * y) % 1_000_000);
    //*x = *x * 2;

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
