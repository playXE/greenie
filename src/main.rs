use greenie::fiber::*;

fn main() {
    let mut f = Fiber::new(|| {
        println!("fiber run!");
    });

    f.start();
}
