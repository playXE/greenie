use crate::detail::ptr::*;
use crate::detail::spinlock::*;
use greenie_ctx::fiber_fcontext::*;
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListLink};
intrusive_adapter! (pub WaitAdapter = Ptr<Context> : Context {wait_hook: LinkedListLink});
intrusive_adapter! (pub TerminatedAdapter = Ptr<Context> : Context {terminated_hook: LinkedListLink});
intrusive_adapter! (pub SleepAdapter = Ptr<Context> : Context {sleep_hook: LinkedListLink});
intrusive_adapter! (pub ReadyAdapter = Ptr<Context> : Context {ready_hook: LinkedListLink});
intrusive_adapter! (pub WorkerAdapter = Ptr<Context> : Context {worker_hook: LinkedListLink});
#[cfg(not(feature = "no-atomics"))]
intrusive_adapter!(
    pub RemoteReadyAdapter = Ptr<Context> : Context {remote_ready_hook: LinkedListLink}
);

pub struct Context {
    wait_hook: LinkedListLink,
    sleep_hook: LinkedListLink,
    ready_hook: LinkedListLink,
    terminated_hook: LinkedListLink,
    worker_hook: LinkedListLink,
    pub c: greenie_ctx::fiber_fcontext::Fiber,
    pub tp: std::time::Instant,

    pub twstatus: std::sync::atomic::AtomicIsize,
    #[cfg(not(feature = "not-atomics"))]
    remote_ready_hook: LinkedListLink,
    splk: crate::detail::spinlock::SpinLock,
    wait_queue: LinkedList<WaitAdapter>,
    pub scheduler: Ptr<crate::scheduler::Scheduler>,
    pub is_main: bool,
    pub is_dispatcher: bool,
    pub terminated: bool,
}

impl Context {
    pub fn new() -> Self {
        Self {
            wait_hook: LinkedListLink::new(),
            sleep_hook: LinkedListLink::new(),
            ready_hook: LinkedListLink::new(),
            terminated_hook: LinkedListLink::new(),
            worker_hook: LinkedListLink::new(),
            c: Fiber {
                fctx: std::ptr::null_mut(),
            },
            tp: std::time::Instant::now(),
            twstatus: std::sync::atomic::AtomicIsize::new(0),
            #[cfg(not(feature = "not-atomics"))]
            remote_ready_hook: LinkedListLink::new(),
            splk: crate::detail::spinlock::SpinLock::new(()),
            wait_queue: LinkedList::new(WaitAdapter::new()),
            scheduler: Ptr::null(),
            is_main: false,
            is_dispatcher: false,
            terminated: false,
        }
    }
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

    pub fn active() -> Ptr<Self> {
        CTX_INIT.with(|x| x.active)
    }

    pub fn resume(&self) {
        let prev = Ptr(self as *const Context as *mut Context);
        CTX_INIT.with(|x| x.active.swap(prev.get()));
        self.c.resume_with(|c| {
            prev.get().c = c;
            Fiber {
                fctx: std::ptr::null_mut(),
            }
        });
    }

    pub fn resume_ctx(&self, ready_ctx: Ptr<Context>) {
        let prev = Ptr(self as *const Context as *mut Context);
        CTX_INIT.with(|x| x.active.swap(prev.get()));
        self.c.resume_with(|c| {
            prev.get().c = c;
            Context::active().schedule(ready_ctx);
            Fiber {
                fctx: std::ptr::null_mut(),
            }
        });
    }

    pub fn resume_lk(&self, lk: SpinLockGuard<'_>) {
        let prev = Ptr(self as *const Context as *mut Context);
        CTX_INIT.with(|x| x.active.swap(prev.get()));
        drop(lk);
        self.c.resume_with(|c| {
            prev.get().c = c;

            Fiber {
                fctx: std::ptr::null_mut(),
            }
        });
    }
    pub fn suspend(&mut self) {
        self.scheduler.get().suspend()
    }

    pub fn suspend_lk(&mut self, lk: SpinLockGuard<'_>) {
        self.scheduler.get().suspend_lk(lk)
    }
    pub fn schedule(&self, ctx: Ptr<Context>) {
        cfg_if::cfg_if!(
            if #[cfg(not(feature="no-atomics"))] {
                if self.scheduler == ctx.scheduler {
                    self.scheduler.get().schedule(ctx);
                } else {
                    ctx.scheduler.get().schedule_from_remote(ctx);
                }
            } else {
                self.scheduler.get().schedule(ctx);
            }
        );
    }

    pub fn join(&mut self) {
        let active = Context::active();
        let lk = self.splk.lock();
        if !self.terminated {
            self.wait_queue.push_front(active);
            active.scheduler.get().suspend_lk(lk)
        }
    }

    pub fn suspend_with_cc(&mut self) -> greenie_ctx::fiber_fcontext::Fiber {
        let prev = Ptr(self as *const Context as *mut Context);
        CTX_INIT.with(|x| x.active.swap(prev.get()));
        return self.c.resume_with(|c| {
            prev.get().c = c;
            Fiber {
                fctx: std::ptr::null_mut(),
            }
        });
    }

    pub fn terminate(&mut self) -> Fiber {
        let p = Ptr(self as *mut Context);
        let lk = self.splk.lock();

        self.terminated = true;

        while let Some(ctx) = self.wait_queue.pop_front() {
            self.schedule(ctx);
        }
        self.wait_queue.fast_clear();
        self.scheduler.get().terminate(lk, p)
    }

    pub fn clear(&mut self) {
        let lk = self.splk.lock();
        if self.is_dispatcher {
            let ctx = self.wait_queue.pop_front().unwrap();
            assert!(ctx.is_main);
        }
        drop(lk);
    }
    pub fn create_worker<F: FnMut() + 'static>(mut f: F) -> Ptr<Self> {
        let mut ctx = Context::new();

        ctx.c = Fiber::new(move |_c| {
            f();
            Context::active().get().terminate();
        });

        Ptr::new(ctx)
    }
}

use crate::scheduler::*;

thread_local! {
    static CTX_INIT: CtxInit = {
        let main_ctx = Ptr::new(Context::new());
        let sched = Ptr::new(Scheduler::new());
        sched.get().main_ctx = main_ctx;
        main_ctx.get().scheduler = sched;
        main_ctx.get().is_main = true;
        let sched_ctx = Fiber::new(|_| {
            let scheduler = Context::active().scheduler;
            scheduler.get().dispatch();
        });
        let scheduler_ctx = Ptr::new(Context::new());
        scheduler_ctx.get().c = sched_ctx;
        scheduler_ctx.get().is_dispatcher = true;
        scheduler_ctx.get().scheduler = sched;
        sched.get().dispatcher_ctx = scheduler_ctx;
        sched.get().algo.awakened(scheduler_ctx);
        CtxInit {active: main_ctx}
    };
}

struct CtxInit {
    active: Ptr<Context>,
}
