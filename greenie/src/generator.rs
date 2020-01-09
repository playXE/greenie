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

use crate::ctx::*;
use crate::ptr::*;
use crate::scheduler::*;

pub struct Generator {
    pub state: Ptr<GeneratorState>,
    pub(crate) complete: std::cell::Cell<bool>,
    pub thread: Ptr<Context>,
    pub to: Ptr<Context>,
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
            let to = rt.active_ctx;
            let thread = rt.get().spawn_not_schelude(closure, args).thread();
            //RUNTIME.with(|rt| rt.get().suspend(thread));
            let generator = Rc::new(Generator {
                state: Ptr::new(GeneratorState::Ready),
                thread,
                to,
                complete: std::cell::Cell::new(false),
                is_join: false,
            });
            thread.get().generator = Some(generator.clone());
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
        RUNTIME.with(|rt| {
            rt.get().resume(self.thread);
            rt.get().switch_without_current();
        });
        //self.thread.get().state = State::Ready;
        /*        while let GeneratorState::Ready = &*self.state.get() {
                    yield_thread();
                }
        */
        if let GeneratorState::Complete(_) = &self.state.get() {
            self.complete.set(true);
        }
        let state = self.state.take();
        //RUNTIME.with(|rt| rt.get().suspend(self.thread));
        Ok(state)
    }
}

/// Yield generator with a value
pub fn generator_yield<T: 'static>(val: T) -> Result<(), &'static str> {
    crate::scheduler::RUNTIME.with(|rt| rt.get().t_yield_generator(val))
}

//// Iterates through generator
#[macro_export]
macro_rules! iterate_generator {
    (for ($x: ident in $generator: expr) $b: block) => {
        loop {
            match $generator.resume() {
                Ok(greenie::generator::GeneratorState::Yielded($x)) => $b,
                Ok(greenie::generator::GeneratorState::Complete($x)) => {
                    break $x
                },
                Err(e) => {
                    eprintln!("{}",e);
                    std::process::exit(1);
                }
                _ => panic!("Unexpected")
            }
        }
    };
    (for ($x: ident in $generator: expr) $b: block and $b2: block) => {
        loop {
            match $generator.resume() {
                Ok(greenie::generator::GeneratorState::Yielded($x)) => $b,
                Ok(greenie::generator::GeneratorState::Complete($x)) => {
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
                Ok(greenie::generator::GeneratorState::Yielded($x)) => $b,
                Ok(greenie::generator::GeneratorState::Complete($x)) => {
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
                Ok(greenie::generator::GeneratorState::Yielded($x)) => $b,
                Ok(greenie::generator::GeneratorState::Complete($x)) => {
                    $b2
                    break;
                },
                Ok(greenie::generator::GeneratorState::Ready) => panic!("impossible"),
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
