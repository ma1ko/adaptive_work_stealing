extern crate rayon;
#[macro_use]
extern crate lazy_static;
extern crate crossbeam;
use crossbeam::channel::*;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
fn main() {
    const NUM_THREADS: usize = 4;
    lazy_static! {
        static ref V: [AtomicUsize; NUM_THREADS] = Default::default();
    }
    lazy_static! {
        static ref C: [(Sender<JobRef>, Receiver<&'static [u64]>); NUM_THREADS] = [
            bounded(NUM_THREADS),
            bounded(NUM_THREADS),
            bounded(NUM_THREADS),
            bounded(NUM_THREADS)
        ];
    }

    let mut numbers: Mutex<Vec<u64>> = Mutex::new((1..1000000).into_iter().collect());
    fn steal(x: usize) -> Option<Box<dyn FnOnce() + Send>> {
        // let steal = |x: usize| -> Option<Box<dyn FnOnce() + Send>> {
        // println!("Thread {} is stealing", x);

        V[x].fetch_add(1, Ordering::Relaxed);
        // c[x].receive();
        // let mut left = do_split_at(numbers, None);
        // Some(Box::new(|| do_work(Mutex::new(left.to_vec()))))
        None
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .steal_callback(steal)
        .build()
        .unwrap();

    pool.install(|| {
        let split = 10;
        let mut left = do_split_at(numbers, Some(split));
        do_stuff_with(&mut left);
    });
}

// split the data in the mutex, put the right part back and return the left
fn do_split_at(mut numbers: Mutex<Vec<u64>>, index: Option<usize>) -> Box<[u64]> {
    loop {
        let mut n = numbers.get_mut().unwrap();
        let index = index.unwrap_or(n.len() / 2);
        let (left, right) = n.split_at_mut(index);
        if left.len() == 0 {
            return Box::new([]);
        }
        let r = &mut right.to_vec();
        n = r;
    }
}

fn do_stuff_with(numbers: &mut [u64]) {
    numbers.iter().map(|x| x * 2);
}
