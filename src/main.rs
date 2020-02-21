#[macro_use]
extern crate lazy_static;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};

// use rayon_logs as rayon;

const NUM_THREADS: usize = 20;
lazy_static! {
    static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
    static ref R: [bool; NUM_THREADS] = Default::default();
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
        Some(())
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
    rayon::scope(|s| {
        loop {
            // check how many other threads need work
            let steal_counter: usize = V[thread_index].swap(0, Ordering::Relaxed);
            if steal_counter > 0 {
                let len = numbers.len();
                if len < 10 {
                    // Why don't we need to send message to all others to abort?
                    /*
                    (0..steal_counter)
                        .into_iter()
                        .for_each(|_| s.spawn_for(|_| {}));
                    */
                    // finish work and return
                    do_the_work(&mut numbers);
                    break;
                } else {
                    let mut chunks = numbers.chunks_mut((numbers.len() / steal_counter) + 1);
                    numbers = chunks.next().unwrap();
                    chunks.for_each(|chunk| {
                        s.spawn_for(move |_| {
                            let idx: usize = rayon::current_thread_index().unwrap();
                            println!("{}, I'm calculating {} elements", idx, chunk.len());
                            // run(chunk) // deadlocking currently
                            do_the_work(&mut *chunk);
                        });
                    });
                }
            }
            // do *some* work, here: 10 elements
            let (left, right) = numbers.split_at_mut(std::cmp::min(numbers.len(), 10));
            do_the_work(left);
            numbers = right;
        }
    });
}

fn do_the_work(data: &mut [u64]) {
    data.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
}
