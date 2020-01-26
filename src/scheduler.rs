use crate::context::*;
use crate::detail::ptr::*;
use crate::detail::spinlock::*;
use intrusive_collections::LinkedList;
use std::sync::atomic::Ordering;
pub struct Scheduler {
    #[cfg(not(feature = "no-atomics"))]
    remote_ready_splk: SpinLock,
    #[cfg(not(feature = "no-atomics"))]
    /// remote ready-queue contains context' signaled by schedulers
    /// running in other threads
    remote_ready_queue: LinkedList<RemoteReadyAdapter>,
    pub algo: Box<dyn crate::algorithm::Algorithm>,
    /// sleep-queue contains context' which have been called
    /// `scheduler::sleep()`
    sleep_queue: LinkedList<SleepAdapter>,
    /// worker-queue contains all context' mananged by this scheduler
    /// except main-context and dispatcher-context
    /// unlink happens on destruction of a context
    worker_queue: LinkedList<WorkerAdapter>,
    /// terminated-queue contains context' which have been terminated
    terminated_queue: LinkedList<TerminatedAdapter>,
    pub dispatcher_ctx: Ptr<Context>,
    pub main_ctx: Ptr<Context>,
    shutdown: bool,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "no-atomics"))]
            remote_ready_splk: SpinLock::new(()),
            #[cfg(not(feature = "no-atomics"))]
            remote_ready_queue: LinkedList::new(RemoteReadyAdapter::new()),
            algo: Box::new(crate::algorithm::shared_work::SharedWork::new()),
            sleep_queue: LinkedList::new(SleepAdapter::new()),
            worker_queue: LinkedList::new(WorkerAdapter::new()),
            terminated_queue: LinkedList::new(TerminatedAdapter::new()),
            dispatcher_ctx: Ptr::null(),
            main_ctx: Ptr::null(),
            shutdown: false,
        }
    }
    #[cfg(not(feature = "no-atomics"))]
    pub fn remote_ready2ready(&mut self) {
        let mut tmp = LinkedList::new(RemoteReadyAdapter::new());
        let lk = self.remote_ready_splk.lock();
        std::mem::swap(&mut self.remote_ready_queue, &mut tmp);
        drop(lk);
        // get context from remote ready-queue
        while let Some(ctx) = tmp.pop_front() {
            // ctx was signaled from remote (other thread)
            // ctx might have been already resumed because of
            // its wait-op. has been already timed out and
            // thus it was already pushed to the ready-queue
            if !ctx.ready_is_linked() {
                self.schedule(ctx);
            }
        }
    }

    pub fn sleep2ready(&mut self) {
        let now = std::time::Instant::now();
        let mut cursor = self.sleep_queue.cursor_mut();
        let mut to_schedule = std::collections::LinkedList::new();
        while !cursor.is_null() {
            let c = cursor.get().unwrap();
            if c.tp <= now {
                let c: Ptr<Context> = cursor.remove().unwrap();
                let prev =
                    c.twstatus
                        .compare_exchange(-1, -2, Ordering::Acquire, Ordering::Relaxed);
                if let Ok(-1) = prev {
                    cursor.move_next();
                    continue;
                }
                to_schedule.push_front(c);
                cursor.move_next();
            } else {
                break;
            }
        }

        while let Some(ctx) = to_schedule.pop_front() {
            self.schedule(ctx);
        }
    }

    pub fn release_terminated(&mut self) {
        while let Some(ctx) = self.terminated_queue.pop_front() {
            ctx.get().clear();
        }
    }

    pub fn dispatch(&mut self) -> greenie_ctx::fiber_fcontext::Fiber {
        loop {
            if self.shutdown {
                self.algo.notify();
                if self.worker_queue.is_empty() {
                    break;
                }
            }

            #[cfg(not(feature = "no-atomics"))]
            {
                self.remote_ready2ready();
            }
            self.sleep2ready();

            let ctx = self.algo.pick_next();
            if !ctx.is_null() {
                ctx.get().resume_ctx(self.dispatcher_ctx);
            }
        }

        return self.main_ctx.get().suspend_with_cc();
    }

    pub fn schedule(&mut self, ctx: Ptr<Context>) {
        assert!(!ctx.is_null());
        if ctx.sleep_is_linked() {
            ctx.sleep_unlink();
        }
        self.algo.awakened(ctx);
    }
    #[cfg(not(feature = "no-atomics"))]
    pub fn schedule_from_remote(&mut self, ctx: Ptr<Context>) {
        assert!(!ctx.is_null());
        let lk = self.remote_ready_splk.lock();
        self.remote_ready_queue.push_front(ctx);
        drop(lk);
        self.algo.notify();
    }

    pub fn terminate(
        &mut self,
        lk: SpinLockGuard<'_>,
        ctx: Ptr<Context>,
    ) -> greenie_ctx::fiber_fcontext::Fiber {
        self.terminated_queue.push_front(ctx);
        ctx.worker_unlink();
        drop(lk);
        self.algo.pick_next().get().suspend_with_cc()
    }

    pub fn yield_(&mut self, ctx: Ptr<Context>) {
        self.algo.pick_next().resume_ctx(ctx);
    }

    pub fn suspend(&mut self) {
        self.algo.pick_next().resume()
    }

    pub fn suspend_lk(&mut self, lk: SpinLockGuard<'_>) {
        self.algo.pick_next().resume_lk(lk)
    }

    pub fn detach_worker_ctx(&mut self, ctx: Ptr<Context>) {
        ctx.worker_unlink();
        ctx.get().scheduler = Ptr::null();
    }

    pub fn attach_worker_ctx(&mut self, ctx: Ptr<Context>) {
        self.worker_queue.push_front(ctx);
        ctx.get().scheduler = Ptr(self as *mut Scheduler);
    }
}
