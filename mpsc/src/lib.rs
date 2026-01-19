//! A multi-producer single-consumer channel optimized for high-throughput scenarios.
//!
//! This module provides a lock-free MPSC channel with batching support for
//! efficient buffer management in network I/O operations.

#![cfg_attr(feature = "simd", feature(portable_simd))]

mod consumer_list;
mod spsc;

use arrayvec::ArrayVec;
use consumer_list::{pop_all, ConsumerList};

#[inline]
#[cold]
fn cold() {}

#[inline]
pub fn likely(b: bool) -> bool {
    if !b {
        cold()
    }
    b
}

#[inline]
pub fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}

/// Consumer side of the MPSC channel.
pub struct Consumer<T> {
    consumer: ConsumerList<usize>,
    pub cached: ArrayVec<usize, 1024>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Consumer<T> {
    /// Pop a single element from the channel.
    /// Returns `None` if the channel is empty.
    pub fn pop(&mut self) -> Option<usize> {
        if unlikely(self.cached.is_empty()) {
            self.sync();
        }
        self.cached.pop()
    }

    /// Returns the number of elements currently cached locally.
    pub fn available_len(&self) -> usize {
        self.cached.len()
    }

    /// Synchronize with producers to fetch new elements.
    pub fn sync(&mut self) {
        pop_all(&mut self.consumer, &mut self.cached);
    }
}

/// Producer side of the MPSC channel.
pub struct Producer<T> {
    elem: spsc::Producer<usize>,
    list: ConsumerList<usize>,
    buffer: ArrayVec<usize, 16>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Producer<T> {
    fn new(elem: spsc::Producer<usize>, list: ConsumerList<usize>) -> Self {
        Self {
            elem,
            buffer: ArrayVec::new(),
            list,
            _marker: std::marker::PhantomData,
        }
    }

    /// Push an element to the channel.
    #[inline(always)]
    pub fn push(&mut self, elem: impl Into<usize>) {
        let elem = elem.into();
        // SAFETY: the buffer is not full since we flush when capacity is reached
        unsafe { self.buffer.push_unchecked(elem) };
        if self.buffer.len() == self.buffer.capacity() {
            self.flush();
        }
    }

    /// Flush all buffered elements to the underlying channel.
    #[inline(never)]
    pub fn flush(&mut self) {
        let _len = self.buffer.len();
        let iter = self.buffer.drain(..);
        let _res = self.elem.enqueue_many(iter);
    }
}

impl<T> Clone for Producer<T> {
    fn clone(&self) -> Self {
        let (p, c) = spsc::channel(self.list.queue_len);
        let list = self.list.clone();
        list.push(c);
        Self::new(p, list)
    }
}

impl<T> Drop for Producer<T> {
    fn drop(&mut self) {
        self.flush();
        self.list.remove(self.elem.id());
    }
}

/// Create a new MPSC channel with the given capacity.
pub fn channel<T>(size: usize) -> (Producer<T>, Consumer<T>) {
    let list = ConsumerList::new(size);
    let (p, c) = spsc::channel(size);
    list.push(c);
    (
        Producer::new(p, list.clone()),
        Consumer {
            consumer: list,
            cached: ArrayVec::new(),
            _marker: std::marker::PhantomData,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        const LEN: usize = 1024 * 4;
        let (producer, mut consumer) = channel::<usize>(LEN);
        let threads = num_cpus::get();
        let mut handles = Vec::new();
        let mut producers = Vec::new();

        for _ in 0..threads {
            producers.push(producer.clone());
        }

        for mut producer in producers {
            let handle = std::thread::spawn(move || {
                for i in 0..LEN {
                    producer.push(i);
                }
            });
            handles.push(handle);
        }

        let mut count = 0;
        while count < LEN * threads {
            if consumer.pop().is_some() {
                count += 1;
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
