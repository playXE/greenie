use std::sync::atomic::AtomicPtr;

#[repr(C)]
pub struct Context {
    pub id: usize,
    pub(crate) stack: Vec<u8>,
    pub(crate) state: State,
    pub(crate) generator: Option<Rc<crate::generator::Generator>>,
    pub sp: *mut u8,
    pub bp: *mut u8,
    pub(crate) handle: crate::ptr::Ptr<JoinHandleInner>,
    pub(crate) twstatus: AtomicPtr<i8>,
    fun: Box<dyn Fn()>,
}

impl Context {
    pub fn new(stack: usize) -> Self {
        Self {
            id: 0,
            stack: vec![0_u8; stack],
            state: State::Available,
            generator: None,
            fun: Box::new(move || {}),
            sp: std::ptr::null_mut(),
            bp: std::ptr::null_mut(),
            handle: crate::ptr::Ptr::null(),
            twstatus: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub(crate) fn apply<F: 'static, A: 'static + ApplyTo<F> + Clone>(&mut self, f: F, args: A) {
        self.fun = Box::new(move || {
            let result: A::Result = args.clone().apply_to(&f);
            let (generator, handle) = crate::scheduler::RUNTIME.with(|rt| {
                (
                    rt.active_ctx.generator.clone(),
                    rt.active_ctx.handle.clone(),
                )
            });
            if generator.is_some() {
                let gen = generator.as_ref().map(|x| x.clone()).unwrap();
                gen.state
                    .set(crate::generator::GeneratorState::Complete(Box::new(result)));
                crate::scheduler::RUNTIME.with(|rt| {
                    rt.get().resume(gen.to);
                });
            } else if !handle.is_null() {
                crate::scheduler::RUNTIME.with(|rt| {
                    rt.get().resume(handle.get().wait);
                });
                handle.get().value = Some(Box::new(result));
            }
        })
    }

    pub fn exec(&mut self) {
        (self.fun)();

        crate::scheduler::RUNTIME.with(|rt| {
            rt.get().switch_without_current();
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum State {
    Available,
    Running,
    Ready,
    Suspended,
    Done,
}

pub(crate) extern "C" fn ctx_function(context: *mut Context) {
    unsafe {
        let ctx = &mut *context;
        ctx.exec();
    }
}

pub trait ApplyTo<F> {
    type Result;
    fn apply_to(self, f: &F) -> Self::Result;
}

impl<F, R> ApplyTo<F> for ()
where
    F: Fn() -> R,
{
    type Result = R;
    fn apply_to(self, f: &F) -> Self::Result {
        f()
    }
}

impl<F, R, A0> ApplyTo<F> for (A0,)
where
    F: Fn(A0) -> R,
{
    type Result = R;
    fn apply_to(self, f: &F) -> Self::Result {
        f(self.0)
    }
}

impl<F, R, A0, A1> ApplyTo<F> for (A0, A1)
where
    F: Fn(A0, A1) -> R,
{
    type Result = R;
    fn apply_to(self, f: &F) -> Self::Result {
        f(self.0, self.1)
    }
}

macro_rules! impl_apply {
    ($($x: literal),*) => {
        paste::item! {
        impl<F, R, A0,A1,$([< A $x>]),*> ApplyTo<F> for (A0, A1,$([< A $x>]),*)
        where
            F: Fn(A0, A1,$([< A $x>]),*) -> R,
        {
            type Result = R;
            fn apply_to(self, f: &F) -> Self::Result {
                f(self.0, self.1, $(
                    self.$x
                ),* )
            }
        }
    }
    };
}

impl_apply!(2);
impl_apply!(2, 3);
impl_apply!(2, 3, 4);
impl_apply!(2, 3, 4, 5);
impl_apply!(2, 3, 4, 5, 6);
impl_apply!(2, 3, 4, 5, 6, 7);
impl_apply!(2, 3, 4, 5, 6, 7, 8);

use crate::ptr::Ptr;
use std::rc::Rc;

pub(crate) struct JoinHandleInner {
    pub value: Option<Box<dyn std::any::Any>>,
    pub thread: Ptr<Context>,
    pub wait: Ptr<Context>,
}
/// An owned permission to join on a thread (block on its termination).
///
///A JoinHandle detaches the associated thread when it is dropped, which means that there is no longer any handle to thread and no way to join on it.
pub struct JoinHandle<T> {
    pub(crate) marker: std::marker::PhantomData<T>,
    pub(crate) inner: crate::ptr::Ptr<JoinHandleInner>,
}

impl<T> JoinHandle<T> {
    pub fn thread_id(&self) -> usize {
        self.inner.get().thread.id
    }

    pub(crate) fn thread(&self) -> Ptr<Context> {
        self.inner.thread
    }
    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<Box<T>, &'static str>
    where
        T: 'static,
    {
        unsafe {
            if let Some(value) = self.inner.0.read().value {
                return Ok(value.downcast().unwrap());
            } else {
                use crate::scheduler;

                scheduler::RUNTIME.with(|rt| {
                    rt.get().switch_without_current();
                    Ok(self.inner.0.read().value.unwrap().downcast().unwrap())
                })
            }
        }
    }
}

use std::hash::{Hash, Hasher};

impl Hash for Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
