use std::rc::Rc;

#[repr(C)]
pub struct Context {
    pub(crate) stack: Vec<u8>,
    pub(crate) ctx: ThreadContext,
    pub(crate) state: State,
    pub(crate) generator: Option<Rc<crate::generator::Generator>>,
    pub sp: *mut u8,
    pub bp: *mut u8,
    fun: Box<dyn Fn()>,
}

impl Context {
    pub fn new<F: 'static, A: 'static + ApplyTo<F> + Clone>(f: F, args: A, stack: usize) -> Self {
        Self {
            stack: vec![0_u8; stack],
            ctx: ThreadContext::default(),
            state: State::Available,
            generator: None,
            fun: Box::new(move || {
                args.clone().apply_to(&f);
            }),
            sp: std::ptr::null_mut(),
            bp: std::ptr::null_mut(),
        }
    }

    pub(crate) fn apply<F: 'static, A: 'static + ApplyTo<F> + Clone>(&mut self, f: F, args: A) {
        self.fun = Box::new(move || {
            args.clone().apply_to(&f);
        })
    }

    pub fn exec(&mut self) {
        (self.fun)();
        self.state = State::Available;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum State {
    Available,
    Running,
    Ready,
    Suspended,
    Sleep(std::time::Duration, std::time::Instant),
}

#[derive(Debug, Default)]
#[repr(C)]
pub(crate) struct ThreadContext {
    pub rsp: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
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
