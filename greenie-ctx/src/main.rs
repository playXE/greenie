use greenie_ctx::fiber_fcontext::*;
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let n = std::time::Instant::now();
    let f = Fiber::new(|_| println!("Hi from fiber"));
    f.resume();
    let n = n.elapsed();
    let ms = n.as_millis();
    let ns = n.as_nanos();
    println!("Done in {} ms or {}ns", ms, ns);
}
