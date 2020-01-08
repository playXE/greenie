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
    pub state: std::cell::Cell<GeneratorState>,
    pub(crate) complete: std::cell::Cell<bool>,
    pub thread_id: usize,
    pub to: usize,
    pub is_join: bool,
}

use std::rc::Rc;

impl Generator {
    /// Spawn generator
    pub fn spawn<F: 'static, A: 'static + crate::ctx::ApplyTo<F> + Clone>(
        closure: F,
        args: A,
    ) -> Rc<Self> {
        crate::scheduler::RUNTIME.with(|rt| {
            let to = rt.current;
            let thread_id = rt.get().spawn(closure, args);
            rt.get().threads[thread_id].get().state = crate::ctx::State::Suspended;
            let generator = Rc::new(Generator {
                state: std::cell::Cell::new(GeneratorState::Ready),
                thread_id,
                to,
                complete: std::cell::Cell::new(false),
                is_join: false,
            });
            rt.get().threads[thread_id].get().generator = Some(generator.clone());
            generator
        })
    }

    /// Resumes the execution of this generator.
    /// This function will resume execution of the generator or start execution if it hasn't already. This call will return back into the
    /// generator's last suspension point, resuming execution from the latest yield. The generator will continue executing until it
    /// either yields or returns, at which point this function will return.
    pub fn resume(&self) -> Result<GeneratorState, &'static str> {
        if self.complete.get() {
            return Err("Generator already complete");
        }
        crate::scheduler::RUNTIME.with(|rt| {
            rt.get().threads[rt.current].get().state = crate::ctx::State::Suspended;
            rt.get().threads[self.thread_id].get().state = crate::ctx::State::Running;
            let old = rt.current;
            rt.get().current = self.thread_id;
            assert!(rt.threads[self.thread_id].generator.is_some());
            unsafe {
                crate::scheduler::switch_stack(
                    &mut rt.get().threads[old].get().sp,
                    rt.threads[self.thread_id].get().sp,
                    rt.get().threads[self.thread_id].get(),
                );
            }
            let state = self.state.take();
            if let GeneratorState::Complete(_) = &state {
                self.complete.set(true);
            }
            Ok(state)
        })
    }
}

pub fn generator_yield<T: 'static>(val: T) -> Result<(), &'static str> {
    crate::scheduler::RUNTIME.with(|rt| rt.get().t_yield_generator(val))
}
/// Complete generator with value
pub fn generator_return<T: 'static>(val: T) -> Result<(), &'static str> {
    crate::scheduler::RUNTIME.with(|rt| rt.get().t_return_generator(val))
}
