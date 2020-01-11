use greenie::{common::barrier::Barrier, greeny_main, Fiber};
#[greeny_main]
fn main() {
    let mut fibers = vec![];
    let barrier = Barrier::new(10);
    for _ in 0..10 {
        let fiber = Fiber::new(move || {
            println!("Before wait");
            barrier.wait();
            println!("After wait");
        });
        fiber.start().unwrap();
        fibers.push(fiber);
    }

    for f in fibers {
        f.join().unwrap();
    }
}
