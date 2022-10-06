use std::{
    future::Future,
    mem,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

use bytes::Bytes;
use futures_util::future::poll_fn;

/// There can be arbitary number of writers and only one reader.
#[derive(Default, Debug)]
pub(crate) struct MpscBytesChannel(Mutex<Inner>);

#[derive(Default, Debug)]
struct Inner {
    is_eof: bool,
    waker: Option<Waker>,
    buffer: Vec<Bytes>,

    /// Set to true if the reader is dropped so that
    /// no new data will be added.
    reader_dropped: bool,
}

/// Methods for the read end
impl MpscBytesChannel {
    /// * `alt_buffer` - it should be an empty buffer and it will be
    ///   swapped with the internal buffers
    ///   if the internal buffer is not empty and `is_eof` is false.
    ///   On eof, it will remain empty.
    pub(crate) fn poll_for_data<'a>(
        &'a self,
        alt_buffer: &'a mut Vec<Bytes>,
        cx: &mut Context<'_>,
    ) -> Poll<()> {
        alt_buffer.clear();

        let mut guard = self.0.lock().unwrap();

        if !guard.buffer.is_empty() {
            mem::swap(&mut guard.buffer, alt_buffer);
            return Poll::Ready(());
        }

        if guard.is_eof {
            return Poll::Ready(());
        }

        let prev_waker = mem::replace(&mut guard.waker, Some(cx.waker().clone()));

        // Release the lock
        drop(guard);

        // Drop prev_waker here to reduce the critical section.
        drop(prev_waker);

        Poll::Pending
    }

    /// * `alt_buffer` - it should be an empty buffer and it will be
    ///   swapped with the internal buffers
    ///   if the internal buffer is not empty and `is_eof` is false.
    ///   On eof, it will remain empty.
    pub(crate) fn wait_for_data<'a>(
        &'a self,
        alt_buffer: &'a mut Vec<Bytes>,
    ) -> impl Future<Output = ()> + 'a {
        poll_fn(move |cx| self.poll_for_data(alt_buffer, cx))
    }

    /// Drop the reader.
    /// After this point, you cannot call poll_for_data.
    pub(crate) fn drop_reader(&self) {
        let mut guard = self.0.lock().unwrap();

        let prev_waker = mem::take(&mut guard.waker);
        let prev_buffer = mem::take(&mut guard.buffer);

        guard.reader_dropped = true;

        // Release the lock
        drop(guard);

        // Drop the waker/buffer here to reduce the critical section
        drop(prev_waker);
        drop(prev_buffer);
    }
}

/// Methods for the write end
impl MpscBytesChannel {
    pub(crate) fn push_bytes(&self, data: Bytes) {
        self.add_more_data(|buffer| buffer.push(data))
    }

    pub(crate) fn add_more_data<F>(&self, callback: F)
    where
        F: FnOnce(&mut Vec<Bytes>),
    {
        let mut guard = self.0.lock().unwrap();

        if guard.reader_dropped {
            return;
        }

        callback(&mut guard.buffer);

        Self::wake_up_reader(guard);
    }

    /// You must not call add_more_data after this call.
    pub(crate) fn mark_eof(&self) {
        let mut guard = self.0.lock().unwrap();

        if guard.reader_dropped {
            return;
        }

        guard.is_eof = true;
        Self::wake_up_reader(guard);
    }

    fn wake_up_reader(mut guard: MutexGuard<'_, Inner>) {
        let waker = guard.waker.take();

        // Release the lock
        drop(guard);

        // Wake after release to reduce critical section
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
