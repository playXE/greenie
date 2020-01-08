use crate::common::mutex::*;
use crate::ctx::*;
use crate::ptr::*;
use crate::scheduler::*;
/// Synchronization primitive that can be used to block a thread, or multiple threads at the same time,
/// until another thread both modifies a shared variable (the condition), and notifies the condition_variable.
#[derive(Clone)]
pub struct Condvar {
    pub(crate) wait_queue: Ptr<std::collections::LinkedList<crate::ptr::Ptr<Context>>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CvStatus {
    NoTimeout = 1,
    Timeout,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            wait_queue: Ptr::new(std::collections::LinkedList::new()),
        }
    }
    /// wait causes the current thread to block until the condition variable is notified or a spurious wakeup occurs, optionally
    /// looping until some predicate is satisfied.
    ///
    /// Atomically unlocks lock, blocks the current executing thread, and adds it to the list of threads waiting on `self`.
    /// The thread will be unblocked when notify_all() or notify_one() is executed. It may also be unblocked spuriously.
    /// When unblocked, regardless of the reason, lock is reacquired and wait exits.
    pub fn wait_for_mutex(&self, m: &Mutex) {
        let active_ctx = RUNTIME.with(|rt| rt.active_ctx);
        self.wait_queue.get().push_back(active_ctx);
        active_ctx.get().twstatus.store(
            self as *const Condvar as *mut i8,
            std::sync::atomic::Ordering::Release,
        );

        m.unlock();

        yield_thread();

        m.lock();
    }
    /// Equivalent to
    /// ```
    /// while !pred() {
    ///       self.wait_for_mutex(m);
    /// }
    /// ```
    pub fn wait_pred(&self, m: &Mutex, mut pred: impl FnMut() -> bool) {
        while !pred() {
            self.wait_for_mutex(m);
        }
    }
    /// If any threads are waiting on this condvar, calling notify_one unblocks one of the waiting threads.
    pub fn notify_one(&self) {
        //let active_ctx = RUNTIME.with(|rt| rt.get().threads[rt.get().current]);

        while let Some(ctx) = self.wait_queue.get().pop_front() {
            let expected = self as *const Condvar as *mut i8;
            if ctx.get().twstatus.compare_exchange(
                expected,
                -1i8 as *mut i8,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            ) == Ok(expected)
            {
                ctx.get().state = State::Ready;
                yield_thread();
                break;
            } else if expected.is_null() {
                ctx.get().state = State::Ready;
                yield_thread();
                break;
            }
        }
    }
    /// Unblocks all threads currently waiting for this condvar.
    pub fn notify_all(&self) {
        //let active_ctx = RUNTIME.with(|rt| rt.get().threads[rt.get().current]);

        while let Some(ctx) = self.wait_queue.get().pop_front() {
            let expected = self as *const Condvar as *mut i8;
            if ctx.get().twstatus.compare_exchange(
                expected,
                -1i8 as *mut i8,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            ) == Ok(expected)
            {
                ctx.get().state = State::Ready;
                yield_thread();
            } else if expected.is_null() {
                ctx.get().state = State::Ready;
                yield_thread();
            }
        }
    }
}

impl Drop for Condvar {
    fn drop(&mut self) {
        unsafe { std::intrinsics::drop_in_place(self.wait_queue.0) }
    }
}
