use crate::ctx::*;
use crate::detail::spinlock::SpinLock;
use crate::ptr::*;
use crate::scheduler::*;
struct MutexInner {
    pub(crate) owner: Ptr<Context>,
    pub(crate) wait_queue: std::collections::LinkedList<crate::ptr::Ptr<Context>>,
    pub(crate) wait_queue_splk: SpinLock,
}

///A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The mutex can also be statically initialized or created via a
/// new constructor.
#[derive(Clone, PartialEq, Eq)]
pub struct Mutex {
    inner: Ptr<MutexInner>,
}

impl Drop for Mutex {
    fn drop(&mut self) {
        unsafe {
            std::intrinsics::drop_in_place(self.inner.0);
        }
    }
}

impl Mutex {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new() -> Self {
        Self {
            inner: Ptr::new(MutexInner {
                owner: Ptr::null(),
                wait_queue: std::collections::LinkedList::new(),
                wait_queue_splk: SpinLock::new(()),
            }),
        }
    }
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// This function will block the local thread until it is available to acquire the mutex. Upon returning, the thread is the only thread with
    /// the lock held.
    ///
    /// ## Panics
    /// Panics if deadlock found
    pub fn lock(&self) {
        let inner = self.inner.get();
        loop {
            let active_ctx = RUNTIME.with(|rt| rt.get().active_ctx);

            let lk = self.inner.wait_queue_splk.lock();
            if active_ctx == inner.owner {
                panic!("greenie: deadlock detected");
            } else if inner.owner.is_null() {
                inner.owner = active_ctx;
                return;
            }
            inner.wait_queue.push_back(active_ctx);
            RUNTIME.with(|rt| {
                rt.get().suspend_thread_not_yield(rt.active_ctx);
                drop(lk);
                rt.get().switch_without_current();
            });
        }
    }
    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then `false` is returned. Otherwise, `true` is returned.
    /// This function does not block.
    pub fn try_lock(&self) -> bool {
        let active_ctx = RUNTIME.with(|rt| rt.get().active_ctx);
        let inner = self.inner.get();
        let lk = self.inner.wait_queue_splk.lock();
        if active_ctx == inner.owner {
            panic!("greenie: deadlock detected");
        } else if inner.owner.is_null() {
            inner.owner = active_ctx;
        }
        drop(lk);

        yield_thread();
        active_ctx == inner.owner
    }
    /// Unlock current mutex.
    ///
    /// ## Panics
    /// Panics if somebody tries to unlock mutex from another thread
    pub fn unlock(&self) {
        let inner = self.inner.get();
        let active_ctx = RUNTIME.with(|rt| rt.get().active_ctx);
        let lk = self.inner.wait_queue_splk.lock();
        if active_ctx != inner.owner {
            panic!("greenie: no privilege to perform the operation");
        }

        inner.owner = Ptr::null();
        if !inner.wait_queue.is_empty() {
            let ctx = inner.wait_queue.pop_front().unwrap();
            //ctx.get().state = State::Ready;
            Context::resume(ctx);
        }
        drop(lk);
    }
}
