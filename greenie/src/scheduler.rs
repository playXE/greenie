extern "C" {
    fn init_stack(_: *mut u8, _: extern "C" fn(*mut Context)) -> *mut u8;
    pub(crate) fn switch_stack(prev: *mut *mut u8, next: *mut u8, self_: *mut Context);
}

use crate::ctx::*;
use crate::ptr::*;
pub struct Scheduler {
    pub stack_size: usize,
    pub(crate) main_ctx: Ptr<Context>,
    pub active_ctx: Ptr<Context>,
    pub current: usize,
    queue: std::collections::LinkedList<Ptr<Context>>,
    pub(crate) terminated_queue: std::collections::LinkedList<Ptr<Context>>,
}

//static mut FIXED_TIMEOUT: preemptive::itimerspec = preemptive::itimerspec::new(0, 0, 0, 100000000);

impl Scheduler {
    /// Create new scheduler instance
    pub fn new() -> Self {
        let base_thread = Ptr::new(Context::new(1024 * 1024 * 2));

        Self {
            main_ctx: base_thread,
            current: 0,
            stack_size: 1024 * 1024 * 2,
            queue: std::collections::LinkedList::new(),
            terminated_queue: std::collections::LinkedList::new(),
            active_ctx: base_thread,
        }
    }

    pub fn run(&mut self) -> ! {
        while self.yield_() {}
        std::process::exit(0);
    }

    pub fn context_switch(&mut self) -> bool {
        self.cleanup();
        if self.queue.is_empty() {
            return false;
        }

        let next = *self.queue.front().unwrap();
        let prev = self.active_ctx;
        self.queue.pop_front();
        if next == prev {
            return false;
        }
        self.queue.push_back(prev);
        self.active_ctx = next;

        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }

        true
    }
    fn cleanup(&mut self) {
        while let Some(context) = self.terminated_queue.pop_front() {
            if !context.is_null() {
                // TODO: Program segfaults and I currently have no idea where it tries to access
                // terminated thread, need to debug it.
                //std::intrinsics::drop_in_place(context.0);

                // Clear stack, should be enough for now ( will free up to 256kb of memory ).
                context.get().stack.clear();
            }
        }
    }

    pub fn switch_without_current(&mut self) -> bool {
        if self.queue.is_empty() {
            return false;
        }

        self.cleanup();
        let next = *self.queue.front().unwrap();
        let prev = self.active_ctx;
        self.queue.pop_front();
        if next == prev {
            return false;
        }
        //self.queue.push_back(prev);
        self.active_ctx = next;

        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }

        true
    }

    pub fn spawn_not_schelude<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> JoinHandle<A::Result> {
        let available = Ptr::new(Context::new(1024 * 1024 * 2));
        let size = available.stack.len();
        let s_ptr = available.get().stack.as_mut_ptr();
        available.get().bp = s_ptr;
        let inner_joinhandle = Ptr::new(JoinHandleInner {
            value: None,
            thread: available,
            wait: self.active_ctx,
        });
        available.get().handle = inner_joinhandle;
        available.get().apply(f, args);
        unsafe {
            available.get().sp =
                init_stack(available.get().bp.offset(size as isize - 128), ctx_function);
        }
        available.get().state = State::Ready;
        available.get().scheduler = Ptr(self as *mut _);
        //self.queue.push_back(available);
        JoinHandle {
            marker: std::marker::PhantomData,
            inner: inner_joinhandle,
        }
    }

    pub fn spawn<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> JoinHandle<A::Result> {
        /*let val = self
            .threads
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.state == State::Available);
        let (_i_, available) = if let Some((i, available)) = val {
            (i, *available)
        } else {
            let thread = Ptr::new(Context::new(1024 * 1024 * 2));
            thread.get().id = self.threads.len();
            self.threads.push(thread);

            (thread.id, thread)
        };*/
        let available = Ptr::new(Context::new(1024 * 1024 * 2));
        let size = available.stack.len();
        let s_ptr = available.get().stack.as_mut_ptr();
        available.get().bp = s_ptr;
        let inner_joinhandle = Ptr::new(JoinHandleInner {
            value: None,
            thread: available,
            wait: self.active_ctx,
        });
        available.get().handle = inner_joinhandle;
        available.get().apply(f, args);
        unsafe {
            available.get().sp =
                init_stack(available.get().bp.offset(size as isize - 128), ctx_function);
        }
        available.get().state = State::Ready;
        available.get().scheduler = Ptr(self as *mut _);
        self.queue.push_back(available);
        JoinHandle {
            marker: std::marker::PhantomData,
            inner: inner_joinhandle,
        }
    }

    pub(crate) fn t_yield_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        if self.active_ctx.generator.is_none() {
            return Err("Not a generator");
        }
        let to = if let Some(generator) = self.active_ctx.get().generator.as_mut() {
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

        self.resume(to);
        self.switch_without_current();

        Ok(())
    }

    pub fn resume(&mut self, t: Ptr<Context>) {
        self.queue.push_back(t);
    }

    pub fn suspend(&mut self, _: Ptr<Context>) {
        self.switch_without_current();
    }

    /// Yield current thread

    pub fn yield_(&mut self) -> bool {
        self.context_switch()
    }
    /// Initialize thread local Scheduler instance
    pub fn init() {
        RUNTIME.with(|rt| {
            *rt.get() = Scheduler::new();
        })
    }
}

thread_local! {
    pub static RUNTIME: Ptr<Scheduler> = {

        Ptr::new(Scheduler::new())
    };

}
/// Yields thread
///
/// This is used when the programmer knows that the thread will have nothing to do for some time, and thus avoid wasting computing time.
pub extern "C" fn yield_thread() {
    RUNTIME.with(|rt| {
        rt.get().yield_();
    })
}
/// Spawns a new thread
///
/// Currently there are no way to specialize stack size for each thread.
///
/// # Example
/// ```rust
/// use greenie::*;
/// fn green_main() {
///    let handle = spawn_greenie(|x, y| x + y, (1, 2));
///
///    println!("{}", handle.join().unwrap());
///}
/// ```
pub fn spawn_greenie<F: 'static, A: 'static + ApplyTo<F> + Clone>(
    f: F,
    args: A,
) -> JoinHandle<A::Result> {
    RUNTIME.with(|rt| rt.get().spawn(f, args))
}
