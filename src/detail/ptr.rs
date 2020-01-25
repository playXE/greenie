use core::cmp::*;
use core::fmt;

#[repr(transparent)]
pub struct Ptr<T: ?Sized>(pub(crate) *mut T);

impl<T: ?Sized> Ptr<T> {
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

impl<T> Ptr<T> {
    pub fn new(x: T) -> Self {
        let ptr = Box::into_raw(Box::new(x));
        Self(ptr)
    }

    pub fn set(&self, val: T) {
        unsafe { self.0.write(val) };
    }

    pub fn swap(&self, val: &mut T) {
        std::mem::swap(self.get(), val);
    }

    pub fn replace(&self, val: T) -> T {
        core::mem::replace(self.get(), val)
    }

    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        Self(core::ptr::null_mut())
    }
}

use core::hash::*;

impl<T> Hash for Ptr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Ptr<T> {}

impl<T> Copy for Ptr<T> {}
impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> core::ops::Deref for Ptr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}

unsafe impl<T> Send for Ptr<T> {}
unsafe impl<T> Sync for Ptr<T> {}

use intrusive_collections::IntrusivePointer;

unsafe impl<T> IntrusivePointer<T> for Ptr<T> {
    unsafe fn from_raw(ptr: *const T) -> Self {
        Self(ptr as *mut T)
    }
    fn into_raw(self) -> *const T {
        self.0
    }
}
