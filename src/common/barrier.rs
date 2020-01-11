use crate::ptr::Ptr;

struct BarrierInner {
    initial: usize,
    current: usize,
    cycle: usize,
    mtx: super::mutex::Mutex,
    cnd: super::condvar::Condvar,
}

/// A barrier enables multiple threads to synchronize the beginning of some computation.
#[derive(Copy, Clone)]
pub struct Barrier {
    inner: Ptr<BarrierInner>,
}

impl Barrier {
    /// Creates a new barrier that can block a given number of threads.
    ///
    /// A barrier will block `n-1` threads which call wait and then wake up all threads at once when the nth thread calls wait.
    pub fn new(n: usize) -> Self {
        Self {
            inner: Ptr::new(BarrierInner {
                initial: n,
                current: n,
                cycle: 0,
                cnd: super::condvar::Condvar::new(),
                mtx: super::mutex::Mutex::new(),
            }),
        }
    }
    /// Blocks the current thread until all threads have rendezvoused here.
    ///
    /// Barriers are re-usable after all threads have rendezvoused once, and can be used continuously.
    ///
    /// A single (arbitrary) thread will receive a barrier wait result that returns true when returning from this function, and all other threads will receive a result that will return false.
    pub fn wait(&self) -> bool {
        self.inner.mtx.lock();
        let cycle = self.inner.cycle;

        self.inner.get().current -= 1;
        if 0 == self.inner.current {
            self.inner.get().cycle += 1;
            self.inner.get().current = self.inner.initial;
            self.inner.mtx.unlock();
            self.inner.cnd.notify_all();
            return true;
        } else {
            self.inner
                .cnd
                .wait_pred(&self.inner.mtx, || cycle != self.inner.cycle);
        }

        self.inner.mtx.unlock();

        false
    }
}
