use crate::ctx::*;
use crate::ptr::*;
use crate::scheduler::*;

struct MutexInner {
    pub(crate) owner: Ptr<Context>,
    pub(crate) wait_queue: std::collections::LinkedList<crate::ptr::Ptr<Context>>,
}

///A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The mutex can also be statically initialized or created via a
/// new constructor. Each mutex has a type parameter which represents the data that it is protecting. The data can only be accessed
/// through the RAII guards returned from lock and try_lock, which guarantees that the data is only ever accessed when the mutex
/// is locked.
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
            }),
        }
    }
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// This function will block the local thread until it is available to acquire the mutex. Upon returning, the thread is the only thread with
    /// the lock held. An RAII guard is returned to allow scoped unlock of the lock. When the guard goes out of scope, the mutex will be
    /// unlocked.
    ///
    /// ## Panics
    /// Panics if deadlock found
    pub fn lock(&self) {
        let inner = self.inner.get();
        loop {
            let active_ctx = RUNTIME.with(|rt| rt.get().active_ctx);
            if active_ctx == inner.owner {
                panic!("greenie: deadlock detected");
            } else if inner.owner.is_null() {
                inner.owner = active_ctx;
                return;
            }
            inner.wait_queue.push_back(active_ctx);
            RUNTIME.with(|rt| {
                rt.get().suspend(rt.active_ctx);
                //rt.get().yield_();
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
        if active_ctx == inner.owner {
            panic!("greenie: deadlock detected");
        } else if inner.owner.is_null() {
            inner.owner = active_ctx;
        }

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
        if active_ctx != inner.owner {
            panic!("greenie: no privilege to perform the operation");
        }

        inner.owner = Ptr::null();
        if !inner.wait_queue.is_empty() {
            let ctx = inner.wait_queue.pop_front().unwrap();
            //ctx.get().state = State::Ready;
            Context::resume(ctx);
        }
    }
}
