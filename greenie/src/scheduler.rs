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
            active_ctx: base_thread,
        }
    }

    pub fn run(&mut self) -> ! {
        while self.yield_() {}
        std::process::exit(0);
    }

    pub fn context_switch(&mut self) -> bool {
        /*let mut pos = self.current;
        while self.threads[pos].get().state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
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
        self.active_ctx = self.threads[self.current];
        unsafe {
            switch_stack(
                &mut self.threads[old_pos].get().sp,
                self.threads[self.current].get().sp,
                self.threads[self.current].get(),
            );
        }*/
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
    pub fn switch_without_current(&mut self) -> bool {
        if self.queue.is_empty() {
            println!("empty");
            return false;
        }

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

        /*self.active_ctx.get().state = State::Suspended;
        to.get().state = State::Running;
        let old = self.active_ctx.id;
        self.active_ctx = to;
        self.current = to.id;
        unsafe {
            switch_stack(&mut self.threads[old].get().sp, to.get().sp, to.get());
        }*/
        /*self.threads[self.current].get().state = State::Ready;
        //self.threads[self.current].get().generator = None;
        to.get().state = State::Running;
        let old_pos = self.current;
        self.current = to.id;
        self.active_ctx = to;
        unsafe {
            switch_stack(&mut self.threads[old_pos].get().sp, to.sp, to.get());
        }*/
        self.resume(to);
        self.switch_without_current();

        Ok(())
    }

    pub fn resume(&mut self, t: Ptr<Context>) {
        /*if t.id == self.current {
            // do nothing
        } else {
            self.threads[t.id].get().state = State::Ready;
        }*/

        self.queue.push_back(t);
    }

    pub fn suspend(&mut self, _: Ptr<Context>) {
        /*if t.id == self.current {
            self.threads[t.id].get().state = State::Suspended;
            self.yield_();
        } else {
            self.threads[t.id].get().state = State::Suspended;
        }*/
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
