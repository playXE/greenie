#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Address(usize);
use super::*;
impl Address {
    #[inline(always)]
    pub fn from(val: usize) -> Address {
        Address(val)
    }

    #[inline(always)]
    pub fn offset_from(self, base: Address) -> usize {
        debug_assert!(self >= base);

        self.to_usize() - base.to_usize()
    }

    #[inline(always)]
    pub fn offset(self, offset: usize) -> Address {
        Address(self.0 + offset)
    }

    #[inline(always)]
    pub fn sub(self, offset: usize) -> Address {
        Address(self.0 - offset)
    }

    #[inline(always)]
    pub fn add_ptr(self, words: usize) -> Address {
        Address(self.0 + words * mem::ptr_width_usize())
    }

    #[inline(always)]
    pub fn sub_ptr(self, words: usize) -> Address {
        Address(self.0 - words * mem::ptr_width_usize())
    }

    #[inline(always)]
    pub fn to_usize(self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn from_ptr<T>(ptr: *const T) -> Address {
        Address(ptr as usize)
    }

    #[inline(always)]
    pub fn to_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[inline(always)]
    pub fn to_mut_ptr<T>(&self) -> *mut T {
        self.0 as *const T as *mut T
    }

    #[inline(always)]
    pub fn null() -> Address {
        Address(0)
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn is_non_null(self) -> bool {
        self.0 != 0
    }

    #[inline(always)]
    pub fn align_page(self) -> Address {
        mem::page_align(self.to_usize()).into()
    }

    #[inline(always)]
    pub fn align_page_down(self) -> Address {
        Address(self.0 & !(mem::page_size() - 1))
    }

    #[inline(always)]
    pub fn is_page_aligned(self) -> bool {
        mem::is_page_aligned(self.to_usize())
    }

    #[inline(always)]
    pub const fn and(self, x: Address) -> Self {
        Self(self.0 & x.0)
    }

    #[inline(always)]
    pub const fn or(self, x: Address) -> Self {
        Self(self.0 | x.0)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", self.to_usize())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", self.to_usize())
    }
}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Address) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Address) -> core::cmp::Ordering {
        self.to_usize().cmp(&other.to_usize())
    }
}

impl From<usize> for Address {
    fn from(val: usize) -> Address {
        Address(val)
    }
}

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
        unsafe {
            let ptr = libc::malloc(core::alloc::Layout::new::<T>().size()) as *mut T;
            ptr.write(x);
            Self(ptr)
        }
    }

    pub fn set(&self, val: T) {
        unsafe { self.0.write(val) };
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
