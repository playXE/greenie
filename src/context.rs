use crate::detail::ptr::*;
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListLink};

intrusive_adapter!(
    WaitAdapter = Ptr<Context> : Context {wait_hook: LinkedListLink},
    SleepAdapter = Ptr<Context> : Context {sleep_hook: LinkedListLink},
    ReadyAdapter = Ptr<Context> : Context {ready_hook: LinkedListLink},
    TerminatedAdapter = Ptr<Context> : Context {terminated_hook: LinkedListLink}
);

#[cfg(not(feature = "no-atomics"))]
intrusive_adapter!(
    pub RemoteReadyAdapter = Ptr<Contex> : Contex {remote_ready_hook: LinkedListLink}
);

pub struct Context {
    wait_hook: LinkedListLink,
    sleep_hook: LinkedListLink,
    ready_hook: LinkedListLink,
    terminated_hook: LinkedListLink,
    worker_hook: LinkedListLink,
    pub c: greenie_ctx::fiber_fcontext::Fiber,
    pub tp: std::time::Instant,
    #[cfg(not(feature = "not-atomics"))]
    pub twstatus: std::sync::atomic::AtomicIsize,
    #[cfg(not(feature = "not-atomics"))]
    remote_ready_hook: LinkedListLink,

    splk: crate::detail::spinlock::SpinLock,
    wait_queue: LinkedList<WaitAdapter>,
}

impl Context {
    pub fn worker_is_linked(&self) -> bool {
        self.worker_hook.is_linked()
    }

    pub fn sleep_is_linked(&self) -> bool {
        self.sleep_hook.is_linked()
    }

    pub fn ready_is_linked(&self) -> bool {
        self.ready_hook.is_linked()
    }

    pub fn terminated_is_linked(&self) -> bool {
        self.terminated_hook.is_linked()
    }

    pub fn wait_is_linked(&self) -> bool {
        self.wait_hook.is_linked()
    }

    #[cfg(not(feature = "no-atomics"))]
    pub fn remote_is_linked(&self) -> bool {
        self.remote_ready_hook.is_linked()
    }

    pub fn worker_unlink(&self) {
        unsafe {
            self.worker_hook.force_unlink();
        }
    }

    pub fn ready_unlink(&self) {
        unsafe { self.ready_hook.force_unlink() }
    }

    pub fn sleep_unlink(&self) {
        unsafe { self.sleep_hook.force_unlink() }
    }

    pub fn wait_unlink(&self) {
        unsafe { self.wait_hook.force_unlink() }
    }

    pub fn terminated_unlink(&self) {
        unsafe { self.terminated_hook.force_unlink() }
    }

    pub fn resume(&self) {
        let mut prev = Ptr(self as *const Context as *mut Context);
        ACTIVE.with(|x| x.swap(&mut prev));
        self.c.resume_with(|c| {
            prev.get().c = c;
        });
    }

    pub fn resume_ctx(&sefl,ready_ctx: Ptr<Context>) {
        let mut prev = Ptr(self as *const Context as *mut Context);
        ACTIVE.with(|x| x.swap(&mut prev));
        self.c.resume_with(|c| {
            prev.get().c = c;
            // TODO: Schedule ready_ctx
        });
    }
}

thread_local! {
    pub const ACTIVE: Ptr<Context> = Ptr::null();
}
