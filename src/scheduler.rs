extern "C" {
    fn init_stack(_: *mut u8, _: extern "C" fn(*mut Context)) -> *mut u8;
    pub(crate) fn switch_stack(prev: *mut *mut u8, next: *mut u8, self_: *mut Context);
}

use crate::ctx::*;
use crate::ptr::*;
pub struct Scheduler {
    pub stack_size: usize,
    pub(crate) main_ctx: Ptr<Context>,
    pub(crate) dispatcher_ctx: Ptr<Context>,
    pub active_ctx: Ptr<Context>,
    pub current: usize,
    pub(crate) terminated_queue: std::collections::LinkedList<Ptr<Context>>,
    pub(crate) algo: Box<dyn crate::algorithm::Algorithm>,
    pub shutdown: bool,
}

//static mut FIXED_TIMEOUT: preemptive::itimerspec = preemptive::itimerspec::new(0, 0, 0, 100000000);

impl Scheduler {
    /// Create new scheduler instance
    pub fn new() -> Self {
        let base_thread = Ptr::new(Context::new(1024 * 1024 * 2));

        Self {
            main_ctx: base_thread,
            dispatcher_ctx: Ptr::new(Context::new(1024 * 1024 * 2)),
            current: 0,
            stack_size: 1024 * 1024 * 2,
            terminated_queue: std::collections::LinkedList::new(),
            active_ctx: base_thread,
            algo: Box::new(crate::algorithm::shared_work::SharedWork::new()),
            shutdown: false,
        }
    }

    fn dispatch(&mut self) {
        loop {
            if self.shutdown {
                self.algo.notify();
                break;
            }

            self.cleanup();

            if !self.yield_() {
                break;
            }
        }
        self.cleanup();
        self.shutdown = true;
        self.algo.notify();
        self.main_ctx = Ptr::null();
        std::process::exit(0);
    }

    pub fn run(&mut self) {
        self.dispatcher_ctx = self
            .spawn(
                || {
                    RUNTIME.with(|rt| {
                        rt.get().dispatch();
                    })
                },
                (),
            )
            .thread();
        self.dispatcher_ctx.get().is_dispatcher = true;
        self.suspend();
    }

    pub fn context_switch(&mut self) -> bool {
        /*if self.queue.is_empty() {
            return false;
        }

        let next = self.queue.front().clone_pointer().unwrap();
        let prev = self.active_ctx;
        self.queue.pop_front();
        if next == prev {
            return false;
        }
        self.queue.push_back(prev);
        self.active_ctx = next;

        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }*/

        let next = self.algo.pick_next();
        if next.is_null() {
            return false;
        }
        self.cleanup();
        let prev = self.active_ctx;

        if next == prev {
            return false;
        }
        self.algo.awakened(self.active_ctx);
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
                //let _ = unsafe { Box::from_raw(context.0) };
                // Clear stack, should be enough for now ( will free up to 256kb of memory ).
                context.get().stack.clear();
            }
        }
    }

    pub fn switch_without_current(&mut self) -> bool {
        /*if self.queue.is_empty() {
            return false;
        }

        self.cleanup();
        let next = self.queue.front().clone_pointer().unwrap();
        let prev = self.active_ctx;
        self.queue.pop_front();
        if next == prev {
            return false;
        }
        //self.queue.push_back(prev);
        self.active_ctx = next;

        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }*/
        let next = self.algo.pick_next();
        if next.is_null() {
            return false;
        }
        self.cleanup();
        let prev = self.active_ctx;

        if next == prev {
            return false;
        }
        self.active_ctx = next;

        //self.algo.awakened(self.active_ctx);
        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }

        true
    }

    pub fn spawn_not_schedule<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> ThreadHandle<A::Result> {
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
        available.get().scheduler = Ptr(self as *mut _);
        ThreadHandle {
            marker: std::marker::PhantomData,
            inner: inner_joinhandle,
        }
    }

    pub fn spawn<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> ThreadHandle<A::Result> {
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
        available.get().scheduler = Ptr(self as *mut _);
        self.algo.awakened(available);
        ThreadHandle {
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
        self.algo.awakened(t);
    }

    pub fn suspend(&mut self) {
        if Context::active().ready_hook.is_linked() {
            unsafe {
                Context::active().ready_hook.force_unlink();
            }
        }
        self.switch_without_current();
    }

    pub fn suspend_thread(&mut self, thread: Ptr<Context>) {
        if thread.ready_hook.is_linked() {
            unsafe {
                thread.ready_hook.force_unlink();
            }
        }
        self.yield_();
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
) -> ThreadHandle<A::Result> {
    RUNTIME.with(|rt| rt.get().spawn(f, args))
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shutdown = true;

        self.dispatcher_ctx.get().join();
        self.main_ctx = Ptr::null();
    }
}
