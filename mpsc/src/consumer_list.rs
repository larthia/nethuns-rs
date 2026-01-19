//! Consumer list implementation for MPSC channel.

use triomphe::Arc;

use arrayvec::ArrayVec;
use parking_lot::Mutex;

use crate::spsc;

pub(crate) struct ConsumerList<T> {
    list: Arc<Mutex<ArrayVec<spsc::Consumer<T>, 4096>>>,
    pub(crate) queue_len: usize,
}

impl<T> Clone for ConsumerList<T> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            queue_len: self.queue_len,
        }
    }
}

impl<T> ConsumerList<T> {
    pub(crate) fn new(queue_len: usize) -> Self {
        Self {
            list: Arc::new(Mutex::new(ArrayVec::new())),
            queue_len,
        }
    }

    pub(crate) fn push(&self, consumer: spsc::Consumer<T>) {
        self.list.lock().push(consumer);
    }

    pub(crate) fn remove(&mut self, id: usize) {
        let mut list = self.list.lock();
        let len = list.len();
        list.retain(|x| unsafe { x.id() } != id);
        assert!(list.len() == len - 1);
    }

    #[inline(always)]
    pub(crate) fn for_each(&self, mut callback: impl FnMut(&spsc::Consumer<T>)) {
        let tmp = self.list.lock();
        for value in tmp.iter() {
            callback(value);
        }
    }
}

/// Pop all available elements from all producer queues.
#[inline(never)]
#[cold]
pub fn pop_all<const N: usize>(me: &mut ConsumerList<usize>, v: &mut ArrayVec<usize, N>) {
    me.for_each(|consumer| {
        let consumer = unsafe { &mut *consumer.consumer.get() };
        let remaining = v.capacity() - v.len();
        for scan in ringbuf::traits::Consumer::pop_iter(consumer).take(remaining) {
            // SAFETY: we checked remaining capacity above
            unsafe { v.push_unchecked(scan) };
        }
    });
}
