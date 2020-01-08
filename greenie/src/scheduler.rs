#[naked]
#[inline(never)]
unsafe fn switch(
    old: *mut crate::ctx::ThreadContext,
    new: *const crate::ctx::ThreadContext,
    context: *const crate::ctx::Context,
) {
    asm!("
        mov     %rsp, 0x00($0)
        mov     %r15, 0x08($0)
        mov     %r14, 0x10($0)
        mov     %r13, 0x18($0)
        mov     %r12, 0x20($0)
        mov     %rbx, 0x28($0)
        mov     %rbp, 0x30($0)
        mov     0x00($1), %rsp
        mov     0x08($1), %r15
        mov     0x10($1), %r14
        mov     0x18($1), %r13
        mov     0x20($1), %r12
        mov     0x28($1), %rbx
        mov     0x30($1), %rbp
        //movq    $2,%rdi
        ret
        "
    : "=*m"(old)
    : "r"(new),"r"(context)
    );
}

extern "C" {
    fn init_stack(_: *mut u8, _: extern "C" fn(*mut Context)) -> *mut u8;
    pub(crate) fn switch_stack(prev: *mut *mut u8, next: *mut u8, self_: *mut Context);
}

use crate::ctx::*;
use crate::ptr::*;
pub struct Scheduler {
    pub stack_size: usize,
    pub(crate) main_ctx: Ptr<Context>,
    pub(crate) threads: Vec<Ptr<Context>>,
    #[cfg(feature = "preemptive")]
    pub(crate) timer: i32,
    pub current: usize,
}

impl Scheduler {
    pub fn new(max_threads: usize) -> Self {
        let base_thread = Ptr::new(Context::new(|| {}, (), 1024 * 1024 * 2));
        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Ptr<Context>> = (1..max_threads)
            .map(|_| Ptr::new(Context::new(|| {}, (), 1024 * 1024 * 2)))
            .collect();
        threads.append(&mut available_threads);
        Self {
            main_ctx: base_thread,
            threads,
            #[cfg(feature = "preemptive")]
            timer: 0,
            current: 0,
            stack_size: 1024 * 1024 * 2,
        }
    }

    pub fn run(&mut self) -> ! {
        while self.yield_() {}
        std::process::exit(0);
    }

    pub fn context_switch(&mut self) -> bool {
        let mut pos = self.current;
        while self.threads[pos].get().state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
            }
            if let State::Sleep(dur, now) = &self.threads[pos].get().state {
                if now.elapsed() >= *dur {
                    break;
                }
            }
            if pos == self.current {
                return false;
            }
        }

        if self.threads[self.current].get().state != State::Available {
            self.threads[self.current].get().state = State::Ready;
        }

        self.threads[pos].get().state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            /*switch(
                &mut self.threads[old_pos].get().ctx,
                &self.threads[pos].get().ctx,
                &*self.threads[pos].get(),
            );*/
            switch_stack(
                &mut self.threads[old_pos].get().sp,
                self.threads[self.current].get().sp,
                self.threads[self.current].get(),
            );
        }

        true
    }

    pub fn suspend(&mut self, id: usize) {
        if id == self.current {
            self.threads[id].get().state = State::Suspended;
            self.yield_();
        } else {
            self.threads[id].get().state = State::Suspended;
        }
    }

    pub fn resume(&mut self, id: usize) {
        if id == self.current {
            // do nothing
        } else {
            self.threads[id].get().state = State::Ready;
        }
    }

    pub fn spawn<F: 'static, A: 'static + ApplyTo<F> + Clone>(&mut self, f: F, args: A) -> usize {
        let (_i, available) = self
            .threads
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.state == State::Available)
            .expect("no available thread.");
        let size = available.stack.len();
        let s_ptr = available.get().stack.as_mut_ptr();
        available.get().bp = s_ptr;
        available.get().apply(f, args);
        unsafe {
            available.get().sp =
                init_stack(available.get().bp.offset(size as isize - 128), ctx_function);
            //available.get().ctx.rsp = init_stack(s_ptr.offset(-128), ctx_function) as _;
        }
        available.get().state = State::Ready;
        _i
    }

    pub(crate) fn t_yield_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        if self.threads[self.current].generator.is_none() {
            return Err("Not a generator");
        }
        let to = if let Some(generator) = self.threads[self.current].get().generator.as_mut() {
            if generator.is_join {
                return Err("Not a generator");
            }
            let heap_value = Box::new(val);

            generator
                .state
                .set(crate::generator::GeneratorState::Yielded(
                    heap_value as Box<dyn std::any::Any>,
                ));
            generator.to
        } else {
            unreachable!()
        };
        self.threads[self.current].get().state = State::Suspended;
        self.threads[to].get().state = State::Running;
        let old_pos = self.current;
        self.current = to;
        unsafe {
            switch_stack(
                &mut self.threads[old_pos].get().sp,
                self.threads[to].get().sp,
                self.threads[to].get(),
            );
        }
        Ok(())
    }

    pub(crate) fn t_return_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        let to = if let Some(generator) = self.threads[self.current].get().generator.as_mut() {
            let heap_value = Box::new(val);

            generator
                .state
                .set(crate::generator::GeneratorState::Complete(
                    heap_value as Box<dyn std::any::Any>,
                ));
            generator.to
        } else {
            return Err("Can't return value from green thread");
        };

        self.threads[self.current].get().state = State::Available;
        self.threads[self.current].get().generator = None;
        self.threads[to].get().state = State::Running;
        let old_pos = self.current;
        self.current = to;
        unsafe {
            switch_stack(
                &mut self.threads[old_pos].get().sp,
                self.threads[to].sp,
                self.threads[to].get(),
            );
        }
        Ok(())
    }

    #[cfg(not(feature = "preemptive"))]
    pub fn yield_(&mut self) -> bool {
        self.context_switch()
    }

    fn t_return(&mut self) {
        if self.current != 0 {
            self.threads[self.current].get().state = State::Available;
            self.threads[self.current].get().generator = None;
            self.yield_();
        }
    }
}

thread_local! {
    pub static RUNTIME: Ptr<Scheduler> = Ptr::new(Scheduler::new(1024));
}

pub fn yield_thread() {
    RUNTIME.with(|rt| {
        rt.get().yield_();
    })
}

pub fn spawn_greenie<F: 'static, A: 'static + ApplyTo<F> + Clone>(f: F, args: A) {
    RUNTIME.with(|rt| {
        rt.get().spawn(f, args);
    })
}
