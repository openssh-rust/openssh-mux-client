use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

use bytes::Bytes;

/// There can be arbitary number of writers and only one reader.
#[derive(Default, Debug)]
pub(super) struct MpscBytesChannel(Mutex<Inner>);

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
    pub(super) fn wait_for_data<'a>(
        &'a self,
        alt_buffer: &'a mut Vec<Bytes>,
    ) -> impl Future<Output = ()> + 'a {
        struct WaitForData<'a>(&'a MpscBytesChannel, &'a mut Vec<Bytes>);

        impl Future for WaitForData<'_> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                if !guard.buffer.is_empty() {
                    mem::swap(&mut guard.buffer, self.1);
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
        }

        alt_buffer.clear();

        WaitForData(self, alt_buffer)
    }

    /// Drop the reader.
    /// After this point, you cannot call poll_for_data.
    pub(super) fn drop_reader(&self) {
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
    pub(super) fn add_more_data(&self, data: Bytes) {
        let mut guard = self.0.lock().unwrap();

        if guard.reader_dropped {
            return;
        }

        guard.buffer.push(data);
        Self::wake_up_reader(guard);
    }

    /// You must not call add_more_data after this call.
    pub(super) fn mark_eof(&self) {
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
