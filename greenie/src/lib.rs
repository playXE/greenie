#![feature(asm)]
#![feature(naked_functions)]
use std::ptr;
/// 256 kb stack
pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;

// TODO: Maybe make this thing thread_local?
static mut RUNTIME: usize = 0;

pub use greenie_proc::greenify;

pub struct Runtime {
    threads: Vec<Thread>,
    current: usize,
}

#[derive(PartialEq, Eq, Debug)]
pub enum State {
    Available,
    Running,
    Ready,
    Suspended,
    Sleep(std::time::Duration, std::time::Instant),
}

struct Thread {
    id: usize,
    stack: Vec<u8>,
    ctx: ThreadContext,
    state: State,
    generator: Option<Rc<Generator>>,
}

#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
}

impl Thread {
    fn new(id: usize, stack_size: usize) -> Self {
        Thread {
            id,
            stack: vec![0_u8; stack_size],
            ctx: ThreadContext::default(),
            state: State::Available,
            generator: None,
        }
    }
}

impl Runtime {
    pub fn new(max_threads: usize, stack_size: Option<usize>) -> Self {
        let base_thread = Thread {
            id: 0,
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Running,
            generator: None,
        };

        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Thread> = (1..max_threads)
            .map(|i| {
                Thread::new(
                    i,
                    match stack_size {
                        Some(size) => size,
                        _ => DEFAULT_STACK_SIZE,
                    },
                )
            })
            .collect();
        threads.append(&mut available_threads);

        Runtime {
            threads,
            current: 0,
        }
    }

    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    pub fn run(&mut self) -> ! {
        while self.t_yield() {}
        std::process::exit(0);
    }

    fn t_return(&mut self) {
        if self.current != 0 {
            self.threads[self.current].state = State::Available;
            self.threads[self.current].generator = None;
            self.t_yield();
        }
    }

    fn t_yield_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        if self.threads[self.current].generator.is_none() {
            return Err("Not a generator");
        }
        let to = if let Some(generator) = self.threads[self.current].generator.as_mut() {
            if generator.is_join {
                return Err("Not a generator");
            }
            let heap_value = Box::new(val);

            generator.state.set(GeneratorState::Yielded(
                heap_value as Box<dyn std::any::Any>,
            ));
            generator.to
        } else {
            unreachable!()
        };
        self.threads[self.current].state = State::Suspended;
        self.threads[to].state = State::Running;
        let old_pos = self.current;
        self.current = to;
        unsafe {
            switch(&mut self.threads[old_pos].ctx, &self.threads[to].ctx);
        }
        Ok(())
    }
    fn t_return_generator<T: 'static>(&mut self, val: T) -> Result<(), &'static str> {
        let to = if let Some(generator) = self.threads[self.current].generator.as_mut() {
            let heap_value = Box::new(val);

            generator.state.set(GeneratorState::Complete(
                heap_value as Box<dyn std::any::Any>,
            ));
            generator.to
        } else {
            return Err("Can't return value from green thread");
        };

        self.threads[self.current].state = State::Available;
        self.threads[self.current].generator = None;
        self.threads[to].state = State::Running;
        let old_pos = self.current;
        self.current = to;
        unsafe {
            switch(&mut self.threads[old_pos].ctx, &self.threads[to].ctx);
        }
        Ok(())
    }
    pub fn yield_to(&mut self, id: usize) {
        if self.threads[id].state == State::Suspended || self.threads[id].state == State::Ready {
            self.threads[id].state = State::Running;
            /*if self.threads[self.current].state != State::Available {
                self.threads[self.current].state = State::Suspended;
            }*/
            let old_pos = self.current;
            self.current = id;
            unsafe {
                switch(&mut self.threads[old_pos].ctx, &self.threads[id].ctx);
            }
        }
    }

    fn t_yield(&mut self) -> bool {
        let mut pos = self.current;
        while self.threads[pos].state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
            }
            if pos == self.current {
                return false;
            }

            if let State::Sleep(dur, now) = &self.threads[pos].state {
                if now.elapsed() >= *dur {
                    break;
                }
            }
        }

        if self.threads[self.current].state != State::Available {
            self.threads[self.current].state = State::Ready;
        }

        self.threads[pos].state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            switch(&mut self.threads[old_pos].ctx, &self.threads[pos].ctx);
        }

        true
    }

    pub fn suspend(&mut self, id: usize) {
        if id == self.current {
            self.threads[id].state = State::Suspended;
            self.t_yield();
        } else {
            self.threads[id].state = State::Suspended;
        }
    }

    pub fn resume(&mut self, id: usize) {
        if id == self.current {
            // do nothing
        } else {
            self.threads[id].state = State::Ready;
        }
    }

    pub fn spawn(&mut self, f: fn()) -> usize {
        let (i, available) = self
            .threads
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.state == State::Available)
            .expect("no available thread.");

        let size = available.stack.len();
        let s_ptr = available.stack.as_mut_ptr();

        unsafe {
            ptr::write(s_ptr.offset((size - 8) as isize) as *mut u64, guard as u64);
            ptr::write(s_ptr.offset((size - 16) as isize) as *mut u64, f as u64);
            available.ctx.rsp = s_ptr.offset((size - 16) as isize) as u64;
        }
        available.state = State::Ready;
        i
    }
}

#[cfg_attr(any(target_os = "windows", target_os = "linux"), naked)]
fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        let rt = &mut *rt_ptr;
        rt.t_return();
    };
}

/// Yields thread
///
/// This is used when the programmer knows that the thread will have nothing to do for some time, and thus avoid wasting computing time.
pub fn yield_thread() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    };
}

/// W.I.P, doesn't work now
/// Puts the current thread to sleep for at least the specified amount of time.
/// The thread may sleep longer than the duration specified due to scheduling specifics. It will never sleep less.
pub fn thread_sleep(dur: std::time::Duration) {
    let rt = get_rt();
    rt.threads[rt.current].state = State::Sleep(dur, std::time::Instant::now());
    yield_thread();
}
/// Spawns a new thread
///
/// Currently there are no way to specialize stack size for each thread.
pub fn spawn_greenie(f: fn()) {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        let rt = &mut *rt_ptr;
        let _ = rt.spawn(f);
        yield_thread();
    }
}

#[naked]
#[inline(never)]
unsafe fn switch(old: *mut ThreadContext, new: *const ThreadContext) {
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
        ret
        "
    : "=*m"(old)
    : "r"(new)
    :
    : "alignstack" // needed to work on windows
    );
}

fn get_rt() -> &'static mut Runtime {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        &mut *rt_ptr
    }
}
/// The result of a generator resumption.
/// This enum is returned from the Generator::resume method and indicates the possible return values of a generator.
///  Currently this corresponds to either a suspension point (Yielded) or a termination point (Complete).
pub enum GeneratorState {
    /// Generator is ready to work.
    Ready,
    /// The generator suspended with a value.
    /// This state indicates that a generator has been suspended. The value provided in this variant corresponds to the expression passed to yield and allows generators to provide a value each time they yield.
    Yielded(Box<dyn std::any::Any>),
    /// The generator completed with a return value.

    /// This state indicates that a generator has finished execution with the provided value. Once a generator has returned Complete it is considered a programmer error to call resume again.
    Complete(Box<dyn std::any::Any>),
}

impl Default for GeneratorState {
    fn default() -> Self {
        Self::Ready
    }
}
pub struct Generator {
    state: std::cell::Cell<GeneratorState>,
    complete: std::cell::Cell<bool>,
    thread_id: usize,
    to: usize,
    is_join: bool,
}
use std::rc::Rc;
/// Suspend generator with value
pub fn generator_yield<T: 'static>(val: T) -> Result<(), &'static str> {
    let rt = get_rt();
    rt.t_yield_generator(val)
}
/// Complete generator with value
pub fn generator_return<T: 'static>(val: T) -> Result<(), &'static str> {
    let rt = get_rt();
    rt.t_return_generator(val)
}
/// Generators, also commonly referred to as coroutines
impl Generator {
    /// Spawn generator
    pub fn spawn(closure: fn()) -> Rc<Self> {
        let rt = get_rt();

        let to = rt.current;
        let thread_id = rt.spawn(closure);
        rt.threads[thread_id].state = State::Suspended;
        let generator = Rc::new(Generator {
            state: std::cell::Cell::new(GeneratorState::Ready),
            thread_id,
            to,
            complete: std::cell::Cell::new(false),
            is_join: false,
        });
        rt.threads[thread_id].generator = Some(generator.clone());
        generator
    }

    fn spawn_joinable(id: usize) -> Rc<Self> {
        let rt = get_rt();

        let to = rt.current;
        let thread_id = id;
        rt.threads[thread_id].state = State::Suspended;
        let generator = Rc::new(Generator {
            state: std::cell::Cell::new(GeneratorState::Ready),
            thread_id,
            to,
            complete: std::cell::Cell::new(false),
            is_join: true,
        });
        rt.threads[thread_id].generator = Some(generator.clone());
        generator
    }
    /// Resumes the execution of this generator.
    /// This function will resume execution of the generator or start execution if it hasn't already. This call will return back into the
    /// generator's last suspension point, resuming execution from the latest yield. The generator will continue executing until it
    /// either yields or returns, at which point this function will return.
    pub fn resume(&self) -> Result<GeneratorState, &'static str> {
        if self.complete.get() {
            return Err("Generator already complete");
        }
        let rt = get_rt();
        rt.threads[rt.current].state = State::Suspended;
        rt.threads[self.thread_id].state = State::Running;
        let old = rt.current;
        rt.current = self.thread_id;
        assert!(rt.threads[self.thread_id].generator.is_some());
        unsafe {
            switch(&mut rt.threads[old].ctx, &rt.threads[self.thread_id].ctx);
        }
        let state = self.state.take();
        if let GeneratorState::Complete(_) = &state {
            self.complete.set(true);
        }
        Ok(state)
    }
}

//// Iterate through generator:
/// ```rust
/// use greenie::*;
///
/// fn green_main() {
///     let generator = Generator::spawn(|| {
///         generator_yield(1);
///         generator_yield(2);
///         generator_yield(2.5);
///         generator_return("Hello!");
///     });
///
///     let result = iterate_generator! {
///         for (x in generator) {
///             // do something
///         }
///     };
///     println!("{}",result.downcast_ref::<&'static str>().unwrap());
///
/// }
///
///
/// ```
#[macro_export]
macro_rules! iterate_generator {
    (for ($x: ident in $generator: expr) $b: block) => {
        loop {
            match $generator.resume() {
                Ok(GeneratorState::Yielded($x)) => $b,
                Ok(GeneratorState::Complete($x)) => {
                    break $x
                },
                Err(e) => {
                    eprintln!("{}",e);
                    std::process::exit(1);
                }
                _ => unreachable!()
            }
        }
    };
    (for ($x: ident in $generator: expr) $b: block and $b2: block) => {
        loop {
            match $generator.resume() {
                Ok(GeneratorState::Yielded($x)) => $b,
                Ok(GeneratorState::Complete($x)) => {
                    $b2
                    break;
                },
                Ok(GeneratorState::Ready) => panic!("impossible"),
                Err(e) => {
                    eprintln!("{}",e);
                    std::process::exit(1);
                }
                _ => unreachable!()
            }
        }
    };

    (enumerate for ($c: ident, $x: ident in $generator: expr) $b: block) => {
        let mut $c = 0;
        loop {
            match $generator.resume() {
                Ok(GeneratorState::Yielded($x)) => $b,
                Ok(GeneratorState::Complete($x)) => {
                    break $x
                },
                Err(e) => {
                    eprintln!("{}",e);
                    std::process::exit(1);
                }
                _ => unreachable!()
            }
            $c += 1;
        }
    };

    (enumerate for ($c: ident, $x: ident in $generator: expr) $b: block and $b2: block) => {
        let mut $c = 0;
        loop {
            match $generator.resume() {
                Ok(GeneratorState::Yielded($x)) => $b,
                Ok(GeneratorState::Complete($x)) => {
                    $b2
                    break;
                },
                Ok(GeneratorState::Ready) => panic!("impossible"),
                Err(e) => {
                    eprintln!("{}",e);
                    std::process::exit(1);
                }
                _ => unreachable!()
            }
            $c += 1;
        }
    };
}
