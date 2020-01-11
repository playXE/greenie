use crate::ctx::*;
use crate::ptr::*;

use intrusive_collections::LinkedList;

pub struct SharedWork {
    lqueue: LinkedList<ReadyAdapter>,
    rqueue: parking_lot::Mutex<LinkedList<ReadyAdapter>>,
    mtx: parking_lot::Mutex<()>,
    flag: bool,
    suspsend: bool,
    cnd: parking_lot::Condvar,
}

impl SharedWork {
    pub fn new() -> Self {
        Self {
            lqueue: LinkedList::new(ReadyAdapter::new()),
            rqueue: parking_lot::Mutex::new(LinkedList::new(ReadyAdapter::new())),
            mtx: parking_lot::Mutex::new(()),
            flag: false,
            suspsend: false,
            cnd: parking_lot::Condvar::new(),
        }
    }
}

use super::*;

impl Algorithm for SharedWork {
    fn awakened(&mut self, context: Ptr<Context>) {
        if context.is_main || context.is_dispatcher {
            self.lqueue.push_back(context);
        } else {
            self.rqueue.lock().push_back(context);
        }
    }

    fn pick_next(&mut self) -> Ptr<Context> {
        let mut ctx = Ptr::null();
        let mut rqueue = self.rqueue.lock();
        if !rqueue.is_empty() {
            ctx = rqueue.pop_front().unwrap();

            drop(rqueue);
        } else {
            drop(rqueue);
            if !self.lqueue.is_empty() {
                ctx = self.lqueue.pop_front().unwrap();
            }
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
