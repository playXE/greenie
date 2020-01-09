extern "C" {
    fn init_stack(_: *mut u8, _: extern "C" fn(*mut Context)) -> *mut u8;
    pub(crate) fn switch_stack(prev: *mut *mut u8, next: *mut u8, self_: *mut Context);
}

use crate::ctx::*;
use crate::ptr::*;
pub struct Scheduler {
    pub stack_size: usize,
    pub(crate) main_ctx: Ptr<Context>,
    #[cfg(not(feature = "fifo"))]
    pub(crate) threads: Vec<Ptr<Context>>,
    pub active_ctx: Ptr<Context>,
    pub current: usize,
    #[cfg(feature = "preemptive")]
    pub(crate) timer: i32,
    #[cfg(feature = "fifo")]
    pub(crate) queue: std::collections::LinkedList<Ptr<Context>>,
    #[cfg(feature = "fifo")]
    pub(crate) suspended: std::collections::HashSet<Ptr<Context>>,
}

//static mut FIXED_TIMEOUT: preemptive::itimerspec = preemptive::itimerspec::new(0, 0, 0, 100000000);

impl Scheduler {
    /// Create new scheduler instance
    pub fn new(max_threads: usize) -> Self {
        let base_thread = Ptr::new(Context::new(1024 * 1024 * 2));
        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Ptr<Context>> = (1..max_threads)
            .map(|id| {
                let ctx = Ptr::new(Context::new(1024 * 1024 * 2));
                ctx.get().id = id;
                ctx
            })
            .collect();
        threads.append(&mut available_threads);

        Self {
            main_ctx: base_thread,
            #[cfg(not(feature = "fifo"))]
            threads,

            current: 0,
            stack_size: 1024 * 1024 * 2,
            #[cfg(feature = "fifo")]
            queue: std::collections::LinkedList::new(),
            #[cfg(feature = "fifo")]
            suspended: std::collections::HashSet::new(),
            active_ctx: base_thread,
            #[cfg(feature = "preemptive")]
            timer: 0,
        }
    }

    pub fn run(&mut self) -> ! {
        #[cfg(feature = "preemptive")]
        {
            unsafe {
                preemption::enable_preemption(&mut self.timer);
                preemption::reset_timer();
            }
        }
        while self.yield_() {}
        std::process::exit(0);
    }

    #[cfg(feature = "fifo")]
    pub fn context_switch(&mut self) -> bool {
        if self.queue.is_empty() {
            let mut last = Context::new(0);
            unsafe {
                switch_stack(&mut last.sp, self.main_ctx.sp, self.main_ctx.get());
            }
        }
        let next = *self.queue.front().unwrap();
        let mut prev = *self.queue.back().unwrap();
        self.queue.pop_front();
        self.queue.push_back(next);

        if next == prev {
            if next.get().state == State::Running {
                return true;
            }
            if next.get().state == State::Ready {
                let mut last = Context::new(0);
                unsafe {
                    switch_stack(&mut last.sp, next.sp, next.get());
                }
            }
        }

        if prev.get().state == State::Done {
            prev.get().stack.clear();
        }
        self.active_ctx = next;

        unsafe {
            switch_stack(&mut prev.get().sp, next.sp, next.get());
        }
        true
    }
    #[cfg(not(feature = "fifo"))]
    pub fn context_switch(&mut self) -> bool {
        let mut pos = self.current;
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
        }

        true
    }

    #[cfg(not(feature = "fifo"))]
    pub fn spawn<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> JoinHandle<A::Result> {
        let (_i, available) = self
            .threads
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.state == State::Available)
            .expect("no available thread.");
        let size = available.stack.len();
        let s_ptr = available.get().stack.as_mut_ptr();
        available.get().bp = s_ptr;
        let inner_joinhandle = Ptr::new(JoinHandleInner {
            value: None,
            thread: *available,
        });
        available.get().handle = inner_joinhandle;
        available.get().apply(f, args);
        unsafe {
            available.get().sp =
                init_stack(available.get().bp.offset(size as isize - 128), ctx_function);
        }
        available.get().state = State::Ready;
        JoinHandle {
            marker: std::marker::PhantomData,
            inner: inner_joinhandle,
        }
    }

    #[cfg(feature = "fifo")]
    pub fn spawn<F: 'static, A: 'static + ApplyTo<F> + Clone>(
        &mut self,
        f: F,
        args: A,
    ) -> JoinHandle<A::Result> {
        let ctx = Ptr::new(Context::new(1024 * 1024 * 2));
        let size = ctx.stack.len();
        let s_ptr = ctx.get().stack.as_mut_ptr();
        ctx.get().bp = s_ptr;
        let inner_joinhandle = Ptr::new(JoinHandleInner {
            value: None,
            thread: ctx,
        });
        ctx.get().handle = inner_joinhandle;
        ctx.get().apply(f, args);
        unsafe {
            ctx.get().sp = init_stack(ctx.get().bp.offset(size as isize - 128), ctx_function);
        }
        ctx.get().state = State::Ready;
        self.queue.push_back(ctx);
        JoinHandle {
            marker: std::marker::PhantomData,
            inner: inner_joinhandle,
        }
    }
    #[cfg(not(feature = "fifo"))]
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

        /*self.active_ctx.get().state = State::Suspended;
        to.get().state = State::Running;
        let old = self.active_ctx.id;
        self.active_ctx = to;
        self.current = to.id;
        unsafe {
            switch_stack(&mut self.threads[old].get().sp, to.get().sp, to.get());
        }*/
        self.threads[self.current].get().state = State::Ready;
        //self.threads[self.current].get().generator = None;
        to.get().state = State::Running;
        let old_pos = self.current;
        self.current = to.id;
        self.active_ctx = to;
        unsafe {
            switch_stack(&mut self.threads[old_pos].get().sp, to.sp, to.get());
        }
        //self.yield_();
        Ok(())
    }
    #[cfg(feature = "fifo")]
    pub fn resume(&mut self, ctx: Ptr<Context>) {
        if !self.suspended.contains(&ctx) {
            return;
        } else {
            self.suspended.remove(&ctx);
            self.queue.push_back(ctx);
        }
    }

    #[cfg(feature = "fifo")]
    pub fn suspend(&mut self, ctx: Ptr<Context>) {
        self.suspended.insert(ctx);
        self.yield_();
    }

    #[cfg(not(feature = "fifo"))]
    pub fn resume(&mut self, t: Ptr<Context>) {
        if t.id == self.current {
            // do nothing
        } else {
            self.threads[t.id].get().state = State::Ready;
        }
    }
    #[cfg(not(feature = "fifo"))]
    pub fn suspend(&mut self, t: Ptr<Context>) {
        if t.id == self.current {
            self.threads[t.id].get().state = State::Suspended;
            self.yield_();
        } else {
            self.threads[t.id].get().state = State::Suspended;
        }
    }
    #[cfg(not(feature = "fifo"))]
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
        to.get().state = State::Running;
        let old_pos = self.current;
        self.current = to.id;
        self.active_ctx = to;
        unsafe {
            switch_stack(&mut self.threads[old_pos].get().sp, to.sp, to.get());
        }
        Ok(())
    }
    #[cfg(feature = "fifo")]
    pub(crate) fn t_return_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        let to = if let Some(generator) = self.active_ctx.get().generator.as_mut() {
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

        self.active_ctx.get().state = State::Done;
        self.resume(to);
        yield_thread();
        Ok(())
    }
    #[cfg(feature = "fifo")]
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
        self.active_ctx.get().state = State::Suspended;
        self.resume(to);
        let old = self.active_ctx;
        self.active_ctx = to;
        self.current = to.id;
        unsafe {
            switch_stack(&mut old.get().sp, to.get().sp, to.get());
        }
        Ok(())
    }
    /// Yield current thread

    pub fn yield_(&mut self) -> bool {
        #[cfg(feature = "preemptive")]
        {}
        self.context_switch()
    }
    /// Initialize thread local Scheduler instance
    pub fn init(max_threads: usize) {
        RUNTIME.with(|rt| {
            *rt.get() = Scheduler::new(max_threads);
        })
    }
}

thread_local! {
    pub static RUNTIME: Ptr<Scheduler> = {

        Ptr::new(Scheduler::new(1024))
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

#[cfg(feature = "preemptive")]
mod preemption {
    pub type timer_t = i32;
    extern "C" {
        fn __libc_current_sigrtmin() -> c_int;
        pub fn timer_create(
            clock_id: clockid_t,
            evp: *mut sigevent,
            timerid: *mut timer_t,
        ) -> c_int;
        pub fn timer_settime(
            timerid: timer_t,
            flags: c_int,
            value: *const libc::itimerspec,
            ovalue: *mut libc::itimerspec,
        ) -> c_int;
        pub fn timer_delete(timerid: timer_t) -> c_int;
    }

    use libc::{c_int, clockid_t, sigaction, sigevent};
    static mut TS: libc::itimerspec = libc::itimerspec {
        it_interval: libc::timespec {
            tv_nsec: 0,
            tv_sec: 0,
        },
        it_value: libc::timespec {
            tv_sec: 0,
            tv_nsec: 100000000,
        },
    };
    pub unsafe fn reset_timer() {
        super::RUNTIME.with(|rt| {
            unimplemented!()
            //timer_settime(rt.timer, 0, &TS, std::ptr::null_mut());
        });
    }

    pub unsafe fn enable_preemption(id: &mut i32) {
        use super::*;

        let mut sa: sigaction = std::mem::zeroed();
        sa.sa_flags = libc::SA_SIGINFO;
        sa.sa_sigaction = preemptive_sched as usize;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(__libc_current_sigrtmin(), &mut sa, std::ptr::null_mut()) == -1 {
            eprintln!("failed to setup signal handling");
            std::process::exit(1);
        }
        let mut te: sigevent = std::mem::zeroed();
        te.sigev_notify = libc::SIGEV_SIGNAL;
        te.sigev_signo = __libc_current_sigrtmin();
        te.sigev_value.sival_ptr = id as *mut i32 as *mut _;

        println!("{}", timer_create(libc::CLOCK_REALTIME, &mut te, id));
    }

    unsafe fn preemptive_sched(/*_: i32, _: *mut u8, _: *mut u8*/) {
        println!("sched");
    }
}
