extern crate rayon;
#[macro_use]
extern crate lazy_static;
use rayon::send_job;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> Result<(), std::io::Error> {
    const NUM_THREADS: usize = 20;
    lazy_static! {
        static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
    }

    let mut numbers: Vec<u64> = (0..100).into_iter().collect();
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
        let idx: usize = rayon::current_thread_index().unwrap();
        let mut my_part: &mut [u64] = &mut numbers;
        loop {
            let split = 1;
            if split > my_part.len() {
                my_part.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
                break;
            }
            let (left, right) = my_part.split_at_mut(split);
            // do_stuff_with(&mut left);
            // println!("Main is calculating {}", left[0]);
            left.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
            my_part = right;

            // check how many other threads need work
            let ctr: usize = V[idx].swap(0, Ordering::Relaxed);
            if ctr > 0 {
                let len = my_part.len();
                if len < 10 {
                    // if we don't have much left, it's not worth sending that
                    // we need some better way to signal other threads there's no work left.
                    (0..ctr).into_iter().for_each(|_| send_job(Box::new(|| {})));
                } else {
                    // keep (1 / ctr), send (ctr - 1) / ctr to the other threads
                    let (left, right) = my_part.split_at_mut(len / (ctr + 1));
                    println!(
                        "{} are waiting, I'm splitting {} to {}",
                        ctr,
                        left.len(),
                        right.len()
                    );
                    right.chunks_mut(right.len() / ctr).for_each(|chunk| {
                        // This move here is important, or this whole thing crashes...
                        send_job(Box::new(move || {
                            let idx: usize = rayon::current_thread_index().unwrap();
                            println!("{}, I'm calculating {} elements", idx, chunk.len());
                            chunk.iter_mut().for_each(|x: &mut u64| *x = *x * 2);
                        }));
                    });
                    my_part = left; // this threads continues with its part
                }
            }
        }
    });
    // currently syncronisation doesn't work , so we just wait a bit for everybody to finish
    let ten_millis = std::time::Duration::from_millis(100);
    std::thread::sleep(ten_millis);

    assert_eq!(numbers, verify, "wrong output");

    Ok(())
}
