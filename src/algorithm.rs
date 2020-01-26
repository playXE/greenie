pub mod round_robin;
pub mod shared_work;
#[cfg(feature = "atomics")]
pub mod work_stealing;
use crate::ctx::*;
use crate::ptr::*;

pub trait Algorithm {
    fn is_stealing(&self) -> bool {
        false
    }
    fn awakened(&mut self, _: Ptr<Context>);
    fn pick_next(&mut self) -> Ptr<Context>;
    fn notify(&mut self) {}
    fn steal(&mut self) -> Ptr<Context> {
        Ptr::null()
    }
}
