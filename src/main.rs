extern crate rayon;
#[macro_use]
extern crate lazy_static;
use rayon::*;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};

const NUM_THREADS: usize = 5;
lazy_static! {
    static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
}
fn main() -> Result<(), std::io::Error> {
    let mut numbers: Vec<u64> = (0..10000000).into_iter().collect();
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
        // local copy
        let mut numbers: &mut [u64] = &mut numbers;
        let thread_index: usize = rayon::current_thread_index().unwrap();
        rayon::scope(|s| {
            loop {
                // check how many other threads need work
                let steal_counter: usize = V[thread_index].swap(0, Ordering::Relaxed);
                if steal_counter > 0 {
                    let len = numbers.len();
                    if len < 10 {
                        // if we don't have much left, it's not worth sending that
                        // we need some better way to signal other threads there's no work left.

                        (0..steal_counter)
                            .into_iter()
                            .for_each(|_| s.spawn_for(|_| {}));

                        do_the_work(&mut numbers);
                        // finish work and return
                        return;
                    } else {
                        // keep (1 / ctr), send (ctr - 1) / ctr to the other threads
                        let (left, right) = numbers.split_at_mut(len / (steal_counter + 1));
                        println!(
                            "{} waiting, splitting {} to {}",
                            steal_counter,
                            left.len(),
                            right.len()
                        );
                        right
                            .chunks_mut((right.len() / steal_counter) + 1)
                            .for_each(|chunk| {
                                // This move here is important, or this whole thing crashes...
                                // rayon::spawn(move || do_the_work(chunk));
                                s.spawn_for(move |_| {
                                    // send_job(Box::new(move || {
                                    let idx: usize = rayon::current_thread_index().unwrap();
                                    println!("{}, I'm calculating {} elements", idx, chunk.len());
                                    // run(chunk) // deadlocking currently
                                    do_the_work(chunk);
                                });
                            });
                        numbers = left;
                    }
                    // do *some* work, here: 10 elements
                    let (left, right) = numbers.split_at_mut(10);
                    do_the_work(left);
                    numbers = right;
                }
            }
        });
    }); // currently syncronisation doesn't work , so we just wait a bit for everybody to finish
    assert_eq!(numbers, verify, "wrong output");
    Ok(())
}

fn do_the_work(data: &mut [u64]) {
    data.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
}
