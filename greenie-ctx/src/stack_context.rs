use crate::detail::*;
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
pub struct StackContext {
    pub size: usize,
    pub sp: Address,
}
