pub mod shared_work;

use crate::ctx::*;
use crate::ptr::*;

pub trait Algorithm {
    fn awakened(&mut self, _: Ptr<Context>);
    fn pick_next(&mut self) -> Ptr<Context>;
    fn notify(&mut self) {}
}
