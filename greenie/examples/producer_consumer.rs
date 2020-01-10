use common::{condvar::*, Mutex};

use greenie::*;

fn main() {
    create_main(|| {
        let now = std::time::Instant::now();
        let (m, cv) = (Mutex::new(vec![]), Condvar::new());
        let stopped = ptr::Ptr::new(false);
        let flag = stopped.clone();
        let mtx = m.clone();
        let cnd = cv.clone();
        let producer = Fiber::new(move || {
            let mut count = 10;
            while count != 0 {
                yield_thread();
                let mut queue = mtx.lock();
                count -= 1;
                queue.push(count);
            }

            let lock = mtx.lock();

            println!("Producer is done!");

            *flag.get() = true;
            drop(lock);
            cnd.notify_one();
        });

        let consumer = Fiber::new(move || loop {
            yield_thread();
            let mut queue = m.lock();
            cv.wait_pred(&m.mutex, || {
                println!("Consumer: Waiting...");
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
        });
        consumer.start().unwrap();
        thread_sleep(std::time::Duration::from_millis(2));
        producer.start().unwrap();

        producer.join().unwrap();
        consumer.join().unwrap();
        println!("Program finished in {}ms", now.elapsed().as_millis());
    });
}
