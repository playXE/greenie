use crate::*;
use ctx::*;
use ptr::*;
use scheduler::*;

pub struct Fiber<T> {
    handle: ThreadHandle<T>,
}

impl<T> Fiber<T> {
    pub fn new<F: FnOnce() -> T + Clone + 'static>(closure: F) -> Self {
        Self {
            handle: RUNTIME.with(|rt| {
                rt.get()
                    .spawn_not_schedule(|closure, _| closure(), (Box::new(closure), ()))
            }),
        }
    }

    fn get_thread(&self) -> Ptr<Context> {
        self.handle.thread()
    }

    pub fn yield_(&self) {
        crate::yield_thread();
    }

    pub fn start(&self) -> Result<(), &'static str> {
        if self.get_thread().terminated {
            return Err("Fiber terminated");
        }
        self.get_thread().scheduler.get().resume(self.get_thread());
        Ok(())
    }

    pub fn join(self) -> Result<T, &'static str>
    where
        T: 'static,
    {
        match self.handle.join() {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    pub fn suspend(&self) {
        self.get_thread()
            .scheduler
            .get()
            .suspend_thread(self.get_thread());
    }
}
