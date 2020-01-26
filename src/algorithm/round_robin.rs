use crate::ctx::*;
use crate::ptr::*;

use intrusive_collections::LinkedList;

pub struct RoundRobin {
    rqueue: LinkedList<ReadyAdapter>,
    mtx: parking_lot::Mutex<()>,
    flag: bool,
    suspsend: bool,
    cnd: parking_lot::Condvar,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            rqueue: LinkedList::new(ReadyAdapter::new()),
            mtx: parking_lot::Mutex::new(()),
            flag: false,
            suspsend: false,
            cnd: parking_lot::Condvar::new(),
        }
    }
}

use super::*;

impl Algorithm for RoundRobin {
    fn awakened(&mut self, context: Ptr<Context>) {
        self.rqueue.push_back(context);
    }

    fn pick_next(&mut self) -> Ptr<Context> {
        let mut ctx = Ptr::null();
        if !self.rqueue.is_empty() {
            ctx = self.rqueue.pop_front().unwrap();
        }
        ctx
    }

    fn notify(&mut self) {
        if self.suspsend {
            let lk = self.mtx.lock();
            self.flag = true;
            drop(lk);
            self.cnd.notify_all();
        }
    }
}
