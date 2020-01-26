//! Lightweigh green threads & coroutines in stable Rust.
//!
//! # Getting started
//!  
//! Add greenie to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! greenie = "*"
//! ```
//!
//! Create greenie main function in `src/main.rs`:
//! ```ignore
//! use greenie::*;
//!
//! #[greenie_main]
//! fn main() {
//!     
//! }
//! // Or you can invoke `create_main`:
//! fn main() {
//!     create_main(|| {
//!     });
//! }
//!
//! ```
//!
//!
//! # Example
//! Ping-pong program
//! ```ignore
//! use greenie::channel::*;
//!
//! use greenie::{greeny_main, Fiber};
//! #[greeny_main]
//! fn main() {
//!     let chan_1 = Channel::<&'static str>::new(2);
//!     let chan_2 = Channel::<&'static str>::new(2);
//!     let fping = Fiber::new_capture(
//!         |chan_1, chan_2| {
//!             chan_1.send("ping");
//!             println!("{}", chan_2.recv().unwrap());
//!             chan_1.send("ping");
//!             println!("{}", chan_2.recv().unwrap());
//!             chan_1.send("ping");
//!             println!("{}", chan_2.recv().unwrap());
//!         },
//!         (chan_1.clone(), chan_2.clone()),
//!     );
//!     let fpong = Fiber::new_capture(
//!         |chan_1, chan_2| {
//!             chan_2.send("pong");
//!             println!("{}", chan_1.recv().unwrap());
//!             chan_2.send("pong");
//!             println!("{}", chan_1.recv().unwrap());
//!             chan_2.send("pong");
//!             println!("{}", chan_1.recv().unwrap());
//!         },
//!         (chan_1.clone(), chan_2.clone()),
//!     );
//!
//!     fpong.start().unwrap();
//!     fping.start().unwrap();
//! }
//! ```

#![allow(dead_code, improper_ctypes)]

#[macro_use]
extern crate intrusive_collections;

pub mod algorithm;
pub mod asynchronous;
pub mod common;
pub mod ctx;
pub mod detail;
pub mod fiber;
pub mod generator;
pub mod ptr;
pub mod scheduler;
pub use generator::generator_yield;
pub use scheduler::{spawn_greenie, yield_thread};

pub use greenie_proc::{greenify, greeny_main};
/// Puts the current thread to sleep for at least the specified amount of time.
///
/// The thread may sleep longer than the duration specified due to scheduling specifics or platform-dependent functionality. It will
/// never sleep less.
pub fn thread_sleep(duration: std::time::Duration) {
    let now = std::time::Instant::now();

    while duration > now.elapsed() {
        crate::yield_thread();
    }
}

pub use fiber::Fiber;
pub use generator::*;
/// Specify entry point for program that will use greenie.
pub fn create_main(main_fn: fn()) {
    scheduler::RUNTIME.with(|rt| {
        let h = rt.get().spawn(|f, _| f(), (main_fn, ()));

        h.join().unwrap();

        //unsafe { std::intrinsics::drop_in_place(rt.0) };
    });
}

pub fn run_scheduler() {
    scheduler::RUNTIME.with(|x| x.get().run());
}

#[cfg(feature = "atomics")]
lazy_static::lazy_static!(
    pub(crate) static ref SCHEDULERS: parking_lot::Mutex<Vec<ptr::Ptr<scheduler::Scheduler>>> = parking_lot::Mutex::new(vec![]);
);
#[cfg(feature = "atomics")]
pub fn multithreaded_scheduler(main_fn: fn()) -> ! {
    use crate::*;
    use crossbeam_deque::{Injector, Steal, Stealer, Worker};
    use ctx::*;
    use ptr::*;
    use rand::distributions::{Distribution, Uniform};
    use rand::thread_rng;
    use scoped_threadpool::Pool;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    struct Task<'a> {
        task_id: usize,
        local: Segment,
        worker: Worker<Address>,
        injector: &'a Injector<Address>,
        stealers: &'a [Stealer<Address>],
        terminator: &'a Terminator,

        scheduled: usize,
    }
    impl<'a> Task<'a> {
        fn pop(&mut self) -> Option<Address> {
            self.pop_local()
                .or_else(|| self.pop_worker())
                .or_else(|| self.pop_global())
                .or_else(|| self.steal())
        }

        fn pop_local(&mut self) -> Option<Address> {
            if self.local.is_empty() {
                return None;
            }

            let obj = self.local.pop().expect("should be non-empty");
            Some(obj)
        }

        fn pop_worker(&mut self) -> Option<Address> {
            self.worker.pop()
        }

        fn pop_global(&mut self) -> Option<Address> {
            loop {
                let result = self.injector.steal_batch_and_pop(&mut self.worker);

                match result {
                    Steal::Empty => break,
                    Steal::Success(value) => return Some(value),
                    Steal::Retry => continue,
                }
            }

            None
        }

        fn steal(&self) -> Option<Address> {
            if self.stealers.len() == 1 {
                return None;
            }

            let mut rng = thread_rng();
            let range = Uniform::new(0, self.stealers.len());

            for _ in 0..2 * self.stealers.len() {
                let mut stealer_id = self.task_id;

                while stealer_id == self.task_id {
                    stealer_id = range.sample(&mut rng);
                }

                let stealer = &self.stealers[stealer_id];

                loop {
                    match stealer.steal_batch_and_pop(&self.worker) {
                        Steal::Empty => break,
                        Steal::Success(address) => return Some(address),
                        Steal::Retry => continue,
                    }
                }
            }

            None
        }
    }

    pub struct Terminator {
        const_nworkers: usize,
        nworkers: AtomicUsize,
    }

    impl Terminator {
        pub fn new(number_workers: usize) -> Terminator {
            Terminator {
                const_nworkers: number_workers,
                nworkers: AtomicUsize::new(number_workers),
            }
        }

        pub fn try_terminate(&self) -> bool {
            if self.const_nworkers == 1 {
                return true;
            }

            self.decrease_workers();
            thread::sleep(Duration::from_micros(1));
            self.zero_or_increase_workers()
        }

        fn decrease_workers(&self) -> bool {
            self.nworkers.fetch_sub(1, Ordering::SeqCst) == 1
        }

        fn zero_or_increase_workers(&self) -> bool {
            let mut nworkers = self.nworkers.load(Ordering::Relaxed);

            loop {
                if nworkers == 0 {
                    return true;
                }

                let prev_nworkers =
                    self.nworkers
                        .compare_and_swap(nworkers, nworkers + 1, Ordering::SeqCst);

                if nworkers == prev_nworkers {
                    return false;
                }

                nworkers = prev_nworkers;
            }
        }
    }
    type Address = Ptr<Context>;
    struct Segment {
        data: Vec<Address>,
    }
    const SEGMENT_SIZE: usize = 64;
    impl Segment {
        fn new() -> Segment {
            Segment {
                data: Vec::with_capacity(SEGMENT_SIZE),
            }
        }

        fn empty() -> Segment {
            Segment { data: Vec::new() }
        }

        fn with(addr: Address) -> Segment {
            let mut segment = Segment::new();
            segment.data.push(addr);

            segment
        }

        fn has_capacity(&self) -> bool {
            self.data.len() < SEGMENT_SIZE
        }

        fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        fn push(&mut self, addr: Address) {
            debug_assert!(self.has_capacity());
            self.data.push(addr);
        }

        fn pop(&mut self) -> Option<Address> {
            self.data.pop()
        }

        fn len(&mut self) -> usize {
            self.data.len()
        }
    }

    unreachable!()
}
