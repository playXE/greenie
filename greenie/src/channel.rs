use crate::ctx::*;
use crate::ptr::*;

use std::collections::LinkedList;

struct ChannelInner<T> {
    capacity: usize,
    slots: Vec<Option<T>>,
    waiting_producers: LinkedList<Ptr<Context>>,
    waiting_consumers: LinkedList<Ptr<Context>>,
    closed: bool,
    cidx: usize,
    pidx: usize,
}

impl<T> ChannelInner<T> {
    fn is_full_(&self) -> bool {
        self.cidx == ((self.pidx + 1) % self.capacity)
    }

    fn is_empty_(&self) -> bool {
        self.cidx == self.pidx
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn close(&mut self) {
        //let active_ctx = Context::active();

        if !self.closed {
            self.closed = true;

            while let Some(producer_ctx) = self.waiting_producers.pop_front() {
                let expected = self as *const ChannelInner<T>;
                let result = producer_ctx.get().twstatus.compare_exchange(
                    expected as *mut _,
                    -1i8 as *mut i8,
                    std::sync::atomic::Ordering::Acquire,
                    std::sync::atomic::Ordering::Relaxed,
                );
                match result {
                    Ok(_) => {
                        crate::yield_thread();
                        break;
                    }
                    Err(x) => {
                        if x.is_null() {
                            crate::yield_thread();
                            break;
                        }
                    }
                }
            }

            while let Some(consumer_ctx) = self.waiting_consumers.pop_front() {
                let expected = self as *const ChannelInner<T>;
                let result = consumer_ctx.get().twstatus.compare_exchange(
                    expected as *mut _,
                    -1i8 as *mut i8,
                    std::sync::atomic::Ordering::Acquire,
                    std::sync::atomic::Ordering::Relaxed,
                );
                match result {
                    Ok(_) => {
                        crate::yield_thread();
                        break;
                    }
                    Err(x) => {
                        if x.is_null() {
                            crate::yield_thread();
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn push(&mut self, value: T) -> ChannelStatus {
        let active_ctx = Context::active();
        loop {
            if self.is_closed() {
                return ChannelStatus::Closed;
            } else if self.is_full_() {
                self.waiting_producers.push_back(active_ctx);
                active_ctx
                    .twstatus
                    .store(0 as *mut _, std::sync::atomic::Ordering::Release);
                active_ctx.scheduler.get().suspend_thread(active_ctx);
            } else {
                self.slots[self.pidx] = Some(value);
                self.pidx = (self.pidx + 1) % self.capacity;

                while let Some(consumer_ctx) = self.waiting_consumers.pop_front() {
                    let expected = self as *const ChannelInner<T>;
                    let result = consumer_ctx.get().twstatus.compare_exchange(
                        expected as *mut _,
                        -1i8 as *mut i8,
                        std::sync::atomic::Ordering::Acquire,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    match result {
                        Ok(_) => {
                            crate::yield_thread();
                            break;
                        }
                        Err(x) => {
                            if x.is_null() {
                                crate::yield_thread();
                                break;
                            }
                        }
                    }
                }

                return ChannelStatus::Success;
            }
        }
    }

    pub fn pop(&mut self) -> Result<T, ChannelStatus> {
        let active_ctx = Context::active();
        loop {
            if self.is_empty_() {
                if self.is_closed() {
                    return Err(ChannelStatus::Closed);
                } else {
                    self.waiting_consumers.push_back(active_ctx);
                    active_ctx
                        .twstatus
                        .store(0 as *mut _, std::sync::atomic::Ordering::Release);
                    active_ctx.scheduler.get().suspend_thread(active_ctx);
                }
            } else {
                let mut value = None;
                std::mem::swap(&mut self.slots[self.cidx], &mut value);
                self.cidx = (self.cidx + 1) % self.capacity;

                while let Some(producer_ctx) = self.waiting_producers.pop_front() {
                    let expected = self as *const ChannelInner<T>;
                    let result = producer_ctx.get().twstatus.compare_exchange(
                        expected as *mut _,
                        -1i8 as *mut i8,
                        std::sync::atomic::Ordering::Acquire,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    match result {
                        Ok(_) => {
                            crate::yield_thread();
                            break;
                        }
                        _ => {
                            crate::yield_thread();
                            break;
                        }
                    }
                }

                return Ok(value.unwrap());
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelStatus {
    Success,
    Empty,
    Full,
    Closed,
    Timeout,
}

pub struct Channel<T> {
    inner: Ptr<ChannelInner<T>>,
}
use std::rc::Rc;
impl<T> Channel<T> {
    pub fn new(size: usize) -> Rc<Self> {
        Rc::new(Self {
            inner: Ptr::new(ChannelInner {
                capacity: size,
                slots: {
                    let mut v = Vec::with_capacity(size);
                    for _ in 0..size {
                        v.push(None);
                    }
                    v
                },
                waiting_producers: LinkedList::new(),
                waiting_consumers: LinkedList::new(),
                closed: false,
                cidx: 0,
                pidx: 0,
            }),
        })
    }

    pub fn push(&self, value: T) {
        self.inner.get().push(value);
    }

    pub fn pop(&self) -> Result<T, ChannelStatus> {
        self.inner.get().pop()
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed
    }

    pub fn close(&self) {
        self.inner.get().close()
    }
}

impl<T> Drop for ChannelInner<T> {
    fn drop(&mut self) {
        self.slots.clear();
        self.close();
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        let _ = unsafe { Box::from_raw(self.inner.0) };
    }
}
