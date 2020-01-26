pub mod barrier;
pub mod channel;
pub mod condvar;
pub mod mutex;

pub use channel::Channel;

///A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The mutex can also be statically initialized or created via a
/// new constructor. Each mutex has a type parameter which represents the data that it is protecting. The data can only be accessed
/// through the RAII guards returned from lock and try_lock, which guarantees that the data is only ever accessed when the mutex
/// is locked.
pub struct Mutex<T> {
    value: crate::ptr::Ptr<T>,
    pub mutex: mutex::Mutex,
}

impl<T> Clone for Mutex<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            mutex: self.mutex.clone(),
        }
    }
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new(value: T) -> Self {
        Self {
            value: crate::ptr::Ptr::new(value),
            mutex: mutex::Mutex::new(),
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
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.mutex.lock();
        MutexGuard {
            mtx: self.mutex.clone(),
            value: self.value.get(),
        }
    }
    /// Attempts to acquire this lock.
    /// If the lock could not be acquired at this time, then `None` is returned. Otherwise, RAII guard returned is returned.
    /// This function does not block.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.mutex.try_lock() {
            Some(MutexGuard {
                mtx: self.mutex.clone(),
                value: self.value.get(),
            })
        } else {
            None
        }
    }
}

pub struct MutexGuard<'a, T> {
    value: &'a mut T,
    pub(crate) mtx: mutex::Mutex,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mtx.unlock();
    }
}

use std::ops::{Deref, DerefMut};

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}
/// Synchronization primitive that can be used to block a thread, or multiple threads at the same time,
/// until another thread both modifies a shared variable (the condition), and notifies the condition variable.
pub struct Condvar(condvar::Condvar);

impl Condvar {
    #[inline(always)]
    pub fn new() -> Self {
        Self(condvar::Condvar::new())
    }
    /// wait causes the current thread to block until the condition variable is notified or a spurious wakeup occurs, optionally
    /// looping until some predicate is satisfied.
    ///
    /// Atomically unlocks lock, blocks the current executing thread, and adds it to the list of threads waiting on `self`.
    /// The thread will be unblocked when notify_all() or notify_one() is executed. It may also be unblocked spuriously.
    /// When unblocked, regardless of the reason, lock is reacquired and wait exits.
    #[inline(always)]
    pub fn wait_for_mutex<'a, T>(&self, x: &MutexGuard<'a, T>) {
        self.0.wait_for_mutex(&x.mtx);
    }
    /// Equivalent to
    /// ```c
    /// while !pred() {
    ///       self.wait_for_mutex(m);
    /// }
    /// ```
    #[inline(always)]
    pub fn wait_pred<'a, T>(&self, x: &MutexGuard<'a, T>, pred: impl FnMut() -> bool) {
        self.0.wait_pred(&x.mtx, pred);
    }
    /// If any threads are waiting on this condvar, calling notify_one unblocks one of the waiting threads.
    #[inline(always)]
    pub fn notify_one(&self) {
        self.0.notify_one()
    }
    /// Unblocks all threads currently waiting for this condvar.
    #[inline(always)]
    pub fn notify_all(&self) {
        self.0.notify_all()
    }
}
