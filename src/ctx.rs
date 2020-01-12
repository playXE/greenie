use std::sync::atomic::AtomicPtr;

intrusive_adapter!(pub ReadyAdapter = Ptr<Context> : Context {ready_hook: intrusive_collections::LinkedListLink});
intrusive_adapter!(pub RemoteAdapter = Ptr<Context> : Context {remote_hook: intrusive_collections::LinkedListLink});

#[repr(C)]
pub struct Context {
    pub id: usize,
    pub(crate) stack: Vec<u8>,

    pub(crate) generator: Option<Rc<crate::generator::Generator>>,
    pub(crate) sp: *mut u8,
    pub(crate) bp: *mut u8,
    pub(crate) handle: crate::ptr::Ptr<JoinHandleInner>,
    pub(crate) twstatus: AtomicPtr<i8>,
    pub(crate) wait_queue: std::collections::LinkedList<Ptr<Context>>,
    pub scheduler: Ptr<crate::scheduler::Scheduler>,
    pub terminated: bool,
    fun: Box<dyn Fn()>,
    pub(crate) ready_hook: intrusive_collections::LinkedListLink,
    pub(crate) remote_hook: intrusive_collections::LinkedListLink,
    pub is_main: bool,
    pub is_dispatcher: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Registers {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rsp: *mut u8,
    rbp: *mut u8,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

impl Registers {
    pub unsafe fn get() -> Self {
        unimplemented!()
    }
}

impl Context {
    pub fn new(stack: usize) -> Self {
        Self {
            id: 0,
            stack: vec![0_u8; stack],
            generator: None,
            fun: Box::new(move || {}),
            sp: std::ptr::null_mut(),
            bp: std::ptr::null_mut(),
            handle: crate::ptr::Ptr::null(),
            twstatus: AtomicPtr::new(std::ptr::null_mut()),
            wait_queue: std::collections::LinkedList::new(),
            scheduler: Ptr::null(),
            terminated: false,
            ready_hook: intrusive_collections::LinkedListLink::new(),
            remote_hook: intrusive_collections::LinkedListLink::new(),
            is_main: false,
            is_dispatcher: false,
        }
    }

    pub fn resume(this: Ptr<Context>) {
        this.scheduler.get().resume(this);
    }

    pub(crate) fn detach(&self) {
        unsafe {
            if self.ready_hook.is_linked() {
                self.ready_hook.force_unlink();
            }
        }
    }

    /*pub(crate) fn resume_ctx(this: Ptr<Context>, ctx: Ptr<Context>) {
        let prev = this;
    }*/

    pub(crate) fn ready_is_linked(&self) -> bool {
        self.ready_hook.is_linked()
    }

    pub(crate) fn apply<F: 'static, A: 'static + ApplyTo<F> + Clone>(&mut self, f: F, args: A) {
        self.fun = Box::new(move || {
            let result: Result<A::Result, Box<dyn std::any::Any + 'static + Send>> =
                Ok(args.clone().apply_to(&f));
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
                handle.get().value = Some(result.map(|x| Box::new(x) as Box<dyn std::any::Any>));
            }
        })
    }

    pub fn exec(&mut self) {
        (self.fun)();
        while let Some(context) = self.wait_queue.pop_front() {
            self.scheduler.get().resume(context);
        }
        self.terminated = true;
        crate::scheduler::RUNTIME.with(|rt| {
            rt.get().terminated_queue.push_back(Context::active());
            rt.get().switch_without_current();
        })
    }

    pub fn active() -> Ptr<Context> {
        crate::scheduler::RUNTIME.with(|rt| rt.get().active_ctx)
    }

    pub fn get_stack(&self) -> &[u8] {
        &self.stack
    }

    pub unsafe fn get_stack_mut(&mut self) -> &mut [u8] {
        &mut self.stack
    }
    pub fn join(&mut self) {
        let active_ctx = Context::active();
        if active_ctx.0 == self as *mut Context {
            panic!();
        }
        self.wait_queue.push_back(active_ctx);
        active_ctx.scheduler.get().suspend();
    }
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
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9);
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
impl_apply!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);

use crate::ptr::Ptr;
use std::rc::Rc;

pub(crate) struct JoinHandleInner {
    pub value: Option<Result<Box<dyn std::any::Any>, Box<dyn std::any::Any + 'static + Send>>>,
    pub thread: Ptr<Context>,
    pub wait: Ptr<Context>,
}
/// An owned permission to join on a thread (block on its termination).
///
///A JoinHandle detaches the associated thread when it is dropped, which means that there is no longer any handle to thread and no way to join on it.
pub struct ThreadHandle<T> {
    pub(crate) marker: std::marker::PhantomData<T>,
    pub(crate) inner: crate::ptr::Ptr<JoinHandleInner>,
}

impl<T> ThreadHandle<T> {
    pub fn thread_id(&self) -> usize {
        self.inner.get().thread.id
    }

    pub(crate) fn thread(&self) -> Ptr<Context> {
        self.inner.thread
    }
    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<T, Box<dyn std::any::Any + 'static + Send>>
    where
        T: 'static,
    {
        unsafe {
            if let Some(value) = self.inner.0.read().value {
                value.map(|value| *value.downcast().unwrap())
            } else {
                self.inner.thread.get().join();
                self.inner
                    .0
                    .read()
                    .value
                    .unwrap()
                    .map(|value| *value.downcast().unwrap())
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

impl Drop for Context {
    fn drop(&mut self) {
        println!("drop");
    }
}
