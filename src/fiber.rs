use crate::context::*;
use crate::detail::ptr::*;
use crate::scheduler::*;

pub struct Fiber {
    impl_: Ptr<Context>,
}

impl Fiber {
    pub fn new<F: FnMut() + 'static>(f: F) -> Self {
        Self {
            impl_: Context::create_worker(f),
        }
    }

    pub fn start(&self) {
        let ctx = Context::active();
        self.impl_.get().resume_ctx(ctx);
    }
}
