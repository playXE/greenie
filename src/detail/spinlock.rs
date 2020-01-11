cfg_if::cfg_if! {
    if #[cfg(feature="atomics")] {
        pub type SpinLock = parking_lot::Mutex<()>;
        pub type SpinLockLock<'a> = parking_lot::MutexGuard<'a,()>;
        pub type SpinLockVal<T> = parking_lot::Mutex<T>;
    } else {

        pub type SpinLockVal<T> = T;
        #[derive(Copy,Clone,PartialEq,Eq)]
        pub struct SpinLock;

        impl SpinLock {
            pub const fn new(_: ()) -> Self {
                Self
            }

            pub const fn lock(&self) -> SpinLockLock<'_> {
                SpinLockLock {
                    _m: std::marker::PhantomData
                }
            }

        }
        #[derive(Copy,Clone,PartialEq,Eq)]
        pub struct SpinLockLock<'a> {
            _m: std::marker::PhantomData<&'a ()>
        }
    }
}
