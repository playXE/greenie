cfg_if::cfg_if! {
if #[cfg(feature="no-atomics")] {
    pub struct SpinLock;
    pub struct SpinLockGuard<'a> {
        _marker: std::marker::PhantomData<&'a ()>
    }

    impl SpinLock {
        pub const fn new() -> Self { Self }
        pub const fn lock<'a>(&'a self) -> SpinLockGuard<'a> {
            SpinLockGuard {
                _marker: std::marker::PhantomData,
            }
        }
    }
} else {
    pub type SpinLock = parking_lot::Mutex<()>;
    pub type SpinLockGuard<'a> = parking_lot::MutexGuard<'a,()>;
}
}


