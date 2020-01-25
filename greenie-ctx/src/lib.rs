#![no_std]
pub mod detail;
pub mod fiber_fcontext;
#[cfg(feature = "fixedsize-stack")]
pub mod fixedsize_stack;
pub mod stack_context;

pub mod stack {
    #[cfg(feature = "fixedsize-stack")]
    pub use super::fixedsize_stack::DefaultStack;
}
