#![allow(dead_code, improper_ctypes)]

pub mod common;
pub mod ctx;
pub mod generator;
pub mod ptr;
pub mod scheduler;
pub use generator::{generator_return, generator_yield};
pub use scheduler::{spawn_greenie, yield_thread};

pub fn thread_sleep(duration: std::time::Duration) {
    let now = std::time::Instant::now();

    while duration > now.elapsed() {
        crate::yield_thread();
    }
}
