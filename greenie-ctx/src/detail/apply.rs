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
