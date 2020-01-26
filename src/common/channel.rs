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
                        break;
                    }
                    Err(x) => {
                        if x.is_null() {
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
                        Context::resume(consumer_ctx);

                        break;
                    }
                    Err(x) => {
                        if x.is_null() {
                            Context::resume(consumer_ctx);

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
                // Full? Suspend until receiver will receive value from current channel.
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
                            Context::resume(consumer_ctx);

                            break;
                        }
                        Err(x) => {
                            if x.is_null() {
                                Context::resume(consumer_ctx);
                                break;
                            }
                        }
                    }
                }

                return ChannelStatus::Success;
            }
        }
    }

    pub fn try_push(&mut self, value: T) -> ChannelStatus {
        loop {
            if self.is_closed() {
                return ChannelStatus::Closed;
            } else if self.is_full_() {
                return ChannelStatus::Full;
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
                            Context::resume(consumer_ctx);
                            break;
                        }
                        Err(x) => {
                            if x.is_null() {
                                Context::resume(consumer_ctx);

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
                    // Empty? Suspend until sender will send value.
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
                            Context::resume(producer_ctx);
                            break;
                        }
                        _ => {
                            Context::resume(producer_ctx);

                            break;
                        }
                    }
                }

                return Ok(value.unwrap());
            }
        }
    }

    pub fn try_pop(&mut self) -> Result<T, ChannelStatus> {
        loop {
            if self.is_empty_() {
                if self.is_closed() {
                    return Err(ChannelStatus::Closed);
                } else {
                    return Err(ChannelStatus::Empty);
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
                            Context::resume(producer_ctx);
                            break;
                        }
                        _ => {
                            Context::resume(producer_ctx);
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

/// A channel is a communication object using which fibers can communicate with each other.
/// Technically, a channel is a data transfer pipe where data can be passed into or read from.
/// Hence one fiber can send data into a channel, while other fibers can read that data
/// from the same channel.
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

    /// Blocks the current thread until a message is send or the channel is closed.
    ///
    /// If the channel if full and not closed, this call will block until send operation can proceed. If the channel becomes
    /// closed, this call will wake up and return `ChannelStatus::Closed`.
    pub fn send(&self, value: T) -> ChannelStatus {
        self.inner.get().push(value)
    }
    /// Blocks the current thread until a mesasge is received or the channel is empty and closed.
    ///
    /// If the channel is empty and not closed, this call will block until the receive can proceed. If the channel is empty and becomes
    /// closed, this call will wake up and return an `Err(ChannelStatus::Closed)`
    pub fn recv(&self) -> Result<T, ChannelStatus> {
        self.inner.get().pop()
    }

    /// Attempts to send a message into the channel without blocking.
    ///
    /// This method will either send a message into the channel immediately or return an error if the channel is full or disconnected. The
    /// returned error contains the original message.
    pub fn try_send(&self, value: T) -> ChannelStatus {
        self.inner.get().try_push(value)
    }

    /// Attempts to receive a message from the channel without blocking.
    ///
    /// This method will either receive a message from the channel immediately or return an error if the channel is empty.
    pub fn try_recv(&self) -> Result<T, ChannelStatus> {
        self.inner.get().try_pop()
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
