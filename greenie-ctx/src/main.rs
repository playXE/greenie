use greenie_ctx::fiber_fcontext::*;
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let n = std::time::Instant::now();
    let f = Fiber::new(|_| println!("Hi from fiber"));
    let mut x = 0;
    f.resume_with(|c| {
        x += 10;
        println!("resume with");
        c
    });
    let n = n.elapsed();
    let ms = n.as_millis();
    let ns = n.as_nanos();
    println!("Done in {} ms or {}ns {}", ms, ns, x);
}
