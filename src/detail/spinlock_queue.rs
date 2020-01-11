use crate::ctx::*;

use crate::ptr::*;
use intrusive_collections::{LinkedList, LinkedListLink};
pub struct ContextSpinlockQueue {
    buffer: parking_lot::Mutex<LinkedList<ReadyAdapter>>,
}

impl ContextSpinlockQueue {
    pub fn new() -> Self {
        Self {
            buffer: parking_lot::Mutex::new(LinkedList::new(ReadyAdapter::new())),
        }
    }

    pub fn push(&self, value: Ptr<Context>) {
        let mut buffer = self.buffer.lock();
        buffer.push_back(value);
    }

    pub fn pop(&self) -> Ptr<Context> {
        let mut buffer = self.buffer.lock();
        if buffer.is_empty() {
            return Ptr::null();
        } else {
            buffer.pop_front().unwrap()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.lock().is_empty()
    }

    pub fn steal(&self) -> Ptr<Context> {
        let mut buffer = self.buffer.lock();
        let mut c = Ptr::null();
        if !buffer.is_empty() {
            c = buffer.front().clone_pointer().unwrap();
            if c.is_main || c.is_dispatcher {
                return Ptr::null();
            }
        }
        buffer.pop_front();
        c
    }
}
