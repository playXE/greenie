//! Just type that wraps `*mut T` for easy access to `T` contents (unsafe!)

#[repr(transparent)]
pub struct Ptr<T>(pub(crate) *mut T);

impl<T> Ptr<T> {
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.0 }
    }

    pub fn new(x: T) -> Self {
        Self(Box::into_raw(Box::new(x)))
    }

    pub fn from_box(b: Box<T>) -> Self {
        Self(Box::into_raw(b))
    }
}

use std::hash::*;

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

impl<T> std::ops::Deref for Ptr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}