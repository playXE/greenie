use crate::ctx::*;
use crate::detail::spinlock_queue::*;
use crate::ptr::*;
use crate::scheduler::*;
use std::sync::Barrier;

use intrusive_collections::LinkedList;

pub struct WorkStealing {
    id: usize,
    rqueue: ContextSpinlockQueue,
    suspend: bool,
}

lazy_static::lazy_static! {
    static ref BARRIER: Ptr<Barrier> = Ptr::new(Barrier::new(0));
}
pub fn initialize_before_work_stealing(thread_count: usize) {
    *BARRIER.get() = Barrier::new(thread_count);
}

impl WorkStealing {
    pub fn new(id: usize) -> Self {
        let x = Self {
            id,
            rqueue: ContextSpinlockQueue::new(),
            suspend: false,
        };

        BARRIER.wait();

        x
    }
}
use super::Algorithm;
use rand::*;

impl Algorithm for WorkStealing {
    fn is_stealing(&self) -> bool {
        true
    }
    fn steal(&mut self) -> Ptr<Context> {
        self.rqueue.steal()
    }
    fn awakened(&mut self, ctx: Ptr<Context>) {
        if ctx.is_null() {
            panic!();
        }
        if !ctx.is_main && !ctx.is_dispatcher {
            ctx.detach();
        }
        self.rqueue.push(ctx);
    }
    fn pick_next(&mut self) -> Ptr<Context> {
        let mut victim = self.rqueue.pop();
        if !victim.is_null() {
            return victim;
        } else {
            let mut id = 0;
            let mut count = 0;
            let mut size = crate::SCHEDULERS.lock().len();
            let mut rng = rand::thread_rng();
            loop {
                loop {
                    count += 1;
                    id = rng.gen_range(0, size);

                    if id != self.id {
                        break;
                    }
                }
                let mut sched = crate::SCHEDULERS.lock()[id];
                victim = sched.get().steal();

                if !victim.is_null() && count >= size {
                    break;
                }
            }
        }

        victim
    }
}
