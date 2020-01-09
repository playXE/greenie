use greenie::*;
use scheduler::*;
fn main() {
    RUNTIME.with(|rt| {
        rt.get().spawn(green_main, ());
        rt.get().run();
    })
}

use common::{condvar::*, Mutex};

fn green_main() {
    let (m, cv) = (Mutex::new(vec![]), Condvar::new());
    let stopped = ptr::Ptr::new(false);
    let producer = spawn_greenie(
        |mtx, cvar, stopped| {
            let mut count = 10;
            while count != 0 {
                yield_thread();
                let mut queue = mtx.lock();
                count -= 1;
                queue.push(count);
            }

            let lock = mtx.lock();

            println!("Producer is done!");

            *stopped.get() = true;
            drop(lock);
            cvar.notify_one();
        },
        (m.clone(), cv.clone(), stopped),
    );

    let consumer = spawn_greenie(
        |mtx, cvar, stopped| loop {
            yield_thread();
            let mut queue = mtx.lock();
            cvar.wait_pred(&mtx.mutex, || {
                return *stopped.get() || !queue.is_empty();
            });

            while !queue.is_empty() {
                let val = queue.pop().unwrap();
                println!("Consumer poped: {}", val);
            }

            if *stopped.get() {
                println!("Consumer is done");
                break;
            }
        },
        (m.clone(), cv.clone(), stopped),
    );

    producer.join().unwrap();
    consumer.join().unwrap();
}
