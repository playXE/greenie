use crate::detail::*;
use crate::stack_context::*;
use mem::*;

pub struct FixedsizeStack {
    pub size: usize,
}

impl FixedsizeStack {
    pub fn new(size: usize) -> Self {
        Self {
            size: if size == 0 {
                stack_default_size()
            } else {
                size
            },
        }
    }

    pub fn allocate(&self) -> StackContext {
        let pages = (self.size as f32 / page_size() as f32).ceil() as usize;
        let size = (pages + 1) * page_size();
        let vp = commit(size, false);
        let sctx = StackContext {
            size,
            sp: vp.offset(size),
        };
        sctx
    }

    pub fn deallocate(sctx: &StackContext) {
        uncommit(sctx.sp.sub(sctx.size), sctx.size);
    }
}

pub type DefaultStack = FixedsizeStack;
