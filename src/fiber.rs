use crate::*;
use ctx::*;
use ptr::*;
use scheduler::*;

/// Fiber objects are lightweight microthreads which are cooperatively scheduled.
/// Only one can run at a given time and the `Fiber::yield()` or `yield_thread` functions must be used to switch execution from one fiber to another.
pub struct Fiber<T> {
    pub(crate) handle: ThreadHandle<T>,
    pub(crate) started: Ptr<bool>,
}

impl<T> Fiber<T> {
    /// Create new fiber.
    ///
    /// # Example
    /// ```rust
    ///
    /// use greenie::*;
    /// create_main( || {
    ///     let some_variable = 42;
    ///     let fiber = Fiber::new(move || {
    ///         println!("{}",some_variable);
    ///
    ///         return "Complete";
    ///     });
    ///
    ///     fiber.start();
    ///
    ///     println!("{}",fiber.join().unwrap());
    /// });
    /// ```
    pub fn new<F: FnOnce() -> T + Clone + 'static>(closure: F) -> Self {
        Self {
            handle: RUNTIME.with(|rt| {
                rt.get()
                    .spawn_not_schedule(|closure, _| closure(), (Box::new(closure), ()))
            }),
            started: Ptr::new(false),
        }
    }

    /// Creates new fiber. Main difference from `Fiber::new` is that you can
    /// capture some variables from context much easier:
    /// ```rust
    ///
    /// use greenie::*;
    /// create_main( || {
    ///     let some_variable = 42;
    ///     let fiber = Fiber::new_capture( |v,_| {
    ///         println!("{}",v);
    ///
    ///         return "Complete";
    ///     },(some_variable,()));
    ///
    ///     fiber.start();
    ///
    ///     println!("{}",fiber.join().unwrap());
    /// });
    /// ```

    pub fn new_capture<F: 'static, A: 'static + ApplyTo<F, Result = T> + Clone>(
        closure: F,
        args: A,
    ) -> Self {
        Self {
            handle: RUNTIME.with(|rt| rt.get().spawn_not_schedule(closure, args)),
            started: Ptr::new(false),
        }
    }

    fn get_thread(&self) -> Ptr<Context> {
        self.handle.thread()
    }

    pub fn yield_(&self) {
        crate::yield_thread();
    }
    /// Starts or resumes the execution of this fiber.
    pub fn start(&self) -> Result<(), &'static str> {
        if self.get_thread().terminated {
            return Err("Fiber terminated");
        }
        *self.started.get() = true;
        self.get_thread().scheduler.get().resume(self.get_thread());
        Ok(())
    }
    /// Waits for the associated fiber to finish.
    pub fn join(self) -> Result<T, Box<dyn std::any::Any>>
    where
        T: 'static,
    {
        match self.handle.join() {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    /// Returns true if the fiber hasn't ended yet, false if it has already ended.
    pub fn is_alive(&self) -> bool {
        !self.handle.thread().terminated
    }
    /// Pause fiber execution.
    pub fn suspend(&self) {
        self.get_thread()
            .scheduler
            .get()
            .suspend_thread(self.get_thread());
    }
}
