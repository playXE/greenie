pub mod continuation {
    use crate::detail::*;
    use fcontext::*;
    pub fn ctx_exit(t: Transfer) {}
}
use crate::detail::*;
use crate::stack;
use fcontext::*;
pub struct Record {
    pub sctx: crate::stack_context::StackContext,
    pub salloc: crate::stack::DefaultStack,
    pub fun: Ptr<dyn FnMut(Fiber)>,
}

impl Record {
    pub fn destroy(&mut self) {
        stack::DefaultStack::deallocate(&self.sctx);
    }

    pub fn run(&self, fctx: FContext) -> FContext {
        let mut c = Fiber { fctx };
        (self.fun.get())(c);
        core::mem::replace(&mut c.fctx, core::ptr::null_mut())
    }
}

extern "C" fn context_ontop<F: FnMut(Fiber) -> Fiber>(mut t: Transfer) -> Transfer {
    unsafe {
        let p = t.data as *const (F,) as *mut (F,);
        assert!(!p.is_null());
        t.data = core::ptr::null_mut();
        let mut c = Fiber { fctx: t.fctx };
        c = ((*p).0)(c);
        return Transfer {
            fctx: core::mem::replace(&mut c.fctx, core::ptr::null_mut()),
            data: core::ptr::null_mut(),
        };
    }
}

extern "C" fn context_exit(t: Transfer) -> Transfer {
    unsafe {
        let rec = t.data as *mut Record;
        core::ptr::drop_in_place(rec);
        core::mem::zeroed()
    }
}

extern "C" fn context_entry(mut t: Transfer) {
    unsafe {
        let rec = t.data as *const Record;

        t = jump_fcontext(t.fctx, core::ptr::null_mut());
        t.fctx = (*rec).run(t.fctx);

        ontop_fcontext(t.fctx, rec as *mut u8, context_exit);
        core::hint::unreachable_unchecked();
    }
}

unsafe fn create_context1(salloc: stack::DefaultStack, f: Ptr<dyn FnMut(Fiber)>) -> FContext {
    let sctx = salloc.allocate();
    let storage = sctx.sp.to_usize() - core::mem::size_of::<Record>() & !0xff;
    let storage = storage as *mut Record;
    storage.write(Record {
        sctx,
        salloc,
        fun: f,
    });
    let record = storage;
    let stack_top = (storage as usize - 64) as *mut u8;
    let stack_bottom = (sctx.sp.to_usize() - sctx.size) as *mut u8;
    let size = stack_top as usize - stack_bottom as usize;
    let fctx = make_fcontext(stack_top, size, context_entry);

    return jump_fcontext(fctx, record as *mut u8).fctx;
}

unsafe fn create_context2(
    palloc: crate::stack_context::StackContext,
    salloc: stack::DefaultStack,
    f: Ptr<dyn FnMut(Fiber)>,
) -> FContext {
    /*let sctx = salloc.allocate();
    let storage = sctx.sp.to_usize() - core::mem::size_of::<Record>() & !0xff;
    let storage = storage as *mut Record;
    storage.write(Record {
        sctx,
        salloc,
        fun: f,
    });
    let record = storage;
    let stack_top = (storage as usize - 64) as *mut u8;
    let stack_bottom = (sctx.sp.to_usize() - sctx.size) as *mut u8;
    let size = stack_top as usize - stack_bottom as usize;*/
    let storage = ((palloc.sp.to_usize() - core::mem::size_of::<Record>()) & !0xff) as *mut u8;
    storage.cast::<Record>().write(Record {
        sctx: palloc,
        salloc,
        fun: f,
    });
    let rec = storage.cast::<Record>();
    let stack_top = (storage as usize - 64) as *mut u8;
    let stack_bottom = (palloc.sp.to_usize() - palloc.size) as *mut u8;
    let size = stack_top as usize - stack_bottom as usize;
    let fctx = make_fcontext(stack_top, size, context_entry);

    return jump_fcontext(fctx, rec as *mut u8).fctx;
}

impl Drop for Record {
    fn drop(&mut self) {
        self.destroy();
        unsafe {
            libc::free(self.fun.0 as *const u8 as *mut _);
        }
    }
}

pub struct Continuation {
    pub fctx: FContext,
}

impl Continuation {
    pub fn resume(mut self) -> Continuation {
        Continuation {
            fctx: unsafe {
                jump_fcontext(
                    core::mem::replace(&mut self.fctx, core::ptr::null_mut()),
                    core::ptr::null_mut(),
                )
                .fctx
            },
        }
    }

    pub fn resume_with<F: FnMut(Fiber) -> Fiber>(mut self, f: F) -> Continuation {
        let p = (f,);
        return Continuation {
            fctx: unsafe {
                ontop_fcontext(
                    core::mem::replace(&mut self.fctx, core::ptr::null_mut()),
                    (&p) as *const (F,) as *mut u8,
                    context_ontop::<F>,
                )
                .fctx
            },
        };
    }
}

#[derive(Copy, Clone)]
pub struct Fiber {
    pub fctx: FContext,
}

impl Fiber {
    pub fn new<F: FnMut(Fiber) + 'static>(f: F) -> Self {
        Self {
            fctx: unsafe {
                create_context1(
                    stack::DefaultStack::new(crate::detail::mem::stack_default_size()),
                    Ptr(Ptr::new(f).0 as *mut dyn FnMut(Fiber)),
                )
            },
        }
    }

    pub fn resume(mut self) -> Self {
        Fiber {
            fctx: unsafe {
                jump_fcontext(
                    core::mem::replace(&mut self.fctx, core::ptr::null_mut()),
                    core::ptr::null_mut(),
                )
                .fctx
            },
        }
    }

    pub fn resume_with<F: FnMut(Fiber) -> Fiber>(mut self, f: F) -> Self {
        let p = (f,);
        return Self {
            fctx: unsafe {
                ontop_fcontext(
                    core::mem::replace(&mut self.fctx, core::ptr::null_mut()),
                    (&p) as *const (F,) as *mut u8,
                    context_ontop::<F>,
                )
                .fctx
            },
        };
    }

    pub fn is_live(&self) -> bool {
        !self.fctx.is_null()
    }
}
