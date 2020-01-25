use greenie_ctx::fiber_fcontext::*;
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let a = Rc::new(RefCell::new(0));
    let a_ = a.clone();
    let mut f = Fiber::new(move |mut f| {
        let a = &a_;
        let mut b = 1;
        loop {
            f = f.resume();
            let next = *a.borrow() + b;
            *a.borrow_mut() = b;
            b = next;
        }
    });

    for _ in 0..=20 {
        f = f.resume();
        println!("{}", a.borrow());
    }
}
