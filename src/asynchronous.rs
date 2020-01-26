use crate::ctx::Context as FCtx;
use crate::fiber::*;
use crate::ptr::Ptr;
use std::future::Future;
use std::pin::Pin;
use std::task::*;

impl<T> Fiber<T> {
    unsafe fn poll<F: Future>(f: &mut Fiber<F::Output>, future: Pin<&mut F>) -> Poll<F::Output> {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            |ptr| RawWaker::new(ptr, &VTABLE),
            |ptr| unsafe { FCtx::resume(Ptr(&mut *(ptr as *mut FCtx))) },
            |ptr| unsafe { FCtx::resume(Ptr(&mut *(ptr as *mut FCtx))) },
            |_| {},
        );
        let raw_waker = RawWaker::new(f.handle.thread().0 as *const (), &VTABLE);
        let waker = Waker::from_raw(raw_waker);
        let mut ctx = Context::from_waker(&waker);

        future.poll(&mut ctx)
    }
}

impl<T: 'static> Future for Fiber<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if !*self.started.get() {
            self.start().unwrap();
        }
        if self.is_alive() {
            crate::yield_thread();
            return Poll::Pending;
        } else {
            let x = self.handle.future_join();
            Poll::Ready(x)
        }
    }
}
/*
struct Raw<'a, T> {
    pinned: std::marker::PhantomPinned,
    fiber: Fiber<T>,
}

pub struct FiberTask<'a, T> {
    raw: Ptr<Raw<'a, T>>,
}

impl<'a, T> AsRef<Fiber<T>> for FiberTask<'a, T> {
    fn as_ref(&self) -> &Fiber<T> {
        &self.raw.fiber
    }
}

impl<'a, T> AsMut<Fiber<T>> for FiberTask<'a, T> {
    fn as_mut(&mut self) -> &mut Fiber<T> {
        &mut self.raw.get().fiber
    }
}

impl<'a, T> Future for FiberTask<'a, T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.raw.fiber.is_alive() {
            return Poll::Pending;
        } else {
            Poll::Ready(self.raw.fiber.handle.future_join())
        }
    }
}
*/
