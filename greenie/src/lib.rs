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
//!             chan_1.push("ping");
//!             println!("{}", chan_2.pop().unwrap());
//!             chan_1.push("ping");
//!             println!("{}", chan_2.pop().unwrap());
//!             chan_1.push("ping");
//!             println!("{}", chan_2.pop().unwrap());
//!         },
//!         (chan_1.clone(), chan_2.clone()),
//!     );
//!     let fpong = Fiber::new_capture(
//!         |chan_1, chan_2| {
//!             chan_2.push("pong");
//!             println!("{}", chan_1.pop().unwrap());
//!             chan_2.push("pong");
//!             println!("{}", chan_1.pop().unwrap());
//!             chan_2.push("pong");
//!             println!("{}", chan_1.pop().unwrap());
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

pub mod channel;
pub mod common;
pub mod ctx;
pub mod fiber;
pub mod generator;
pub mod ptr;
pub mod scheduler;
pub use generator::generator_yield;
pub use scheduler::{spawn_greenie, yield_thread};

pub use greenie_proc::{greenify, greeny_main};

pub fn thread_sleep(duration: std::time::Duration) {
    let now = std::time::Instant::now();

    while duration > now.elapsed() {
        crate::yield_thread();
    }
}

pub use fiber::Fiber;
pub use generator::*;

pub fn create_main(main_fn: fn()) {
    scheduler::RUNTIME.with(|rt| {
        let _ = rt.get().spawn(|f, _| f(), (main_fn, ()));
        rt.get().run();
    })
}
