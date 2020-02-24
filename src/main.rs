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
    let mut numbers: Vec<u64> = (0..1000000).into_iter().collect();
    let verify: Vec<u64> = numbers.iter().map(|x| x * 2).collect();
    fn steal(x: usize) -> Option<()> {
        if x != 0 {
            return None;
        } // Only steal from 0 currently

        V[x].fetch_add(1, Ordering::Relaxed);
        let backoff = crossbeam::Backoff::new();
        let mut c = 0;
        for _ in 0..8 {
            // wait until the victim has taken the value, check regularly
            backoff.spin();
            c = V[x].load(Ordering::Relaxed);
            if c == 0 {
                return Some(());
            }
        }
        V[x].compare_and_swap(c, c - 1, Ordering::Relaxed);
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
fn run(mut numbers: &mut [u64]) {
    // local copy
    let mut numbers: &mut [u64] = &mut numbers;
    let thread_index: usize = rayon::current_thread_index().unwrap();

    println!("I am {}", thread_index);
    loop {
        // check how many other threads need work
        let steal_counter: usize = V[thread_index].swap(0, Ordering::Relaxed);
        if steal_counter > 0 {
            let len = numbers.len();
            if len < 100 {
                // finish work and return
                do_the_work(&mut numbers);
                break;
            } else {
                let numbers_len = numbers.len();
                let mut chunks = numbers.chunks_mut(numbers_len / (steal_counter + 1/* for me */));
                println!(
                    "{}: {} steals, total {}",
                    thread_index, steal_counter, numbers_len
                );
                numbers = chunks.next().unwrap();
                fn spawn(chunks: &mut std::slice::ChunksMut<u64>) -> Option<()> {
                    let chunk = chunks.next()?;
                    rayon::join(
                        || {
                            spawn(chunks);
                        },
                        || {
                            do_the_work(chunk);
                        },
                    );
                    None
                };
                spawn(&mut chunks);
            }
        }
        // do *some* work, here: 10 elements
        let (left, right) = numbers.split_at_mut(std::cmp::min(numbers.len(), 100));
        do_the_work(left);
        numbers = right;
    }
}

fn do_the_work(data: &mut [u64]) {
    data.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
}
