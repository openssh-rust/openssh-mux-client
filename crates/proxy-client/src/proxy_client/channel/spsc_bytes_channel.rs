use std::{
    mem,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

use bytes::Bytes;

/// There will be only one writer and only one reader.
#[derive(Default, Debug)]
pub(super) struct SpscBytesChannel(Mutex<Inner>);

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
impl SpscBytesChannel {
    /// * `alt_buffer` - it should be an empty buffer and it will be
    ///   swapped with the internal buffers
    ///   if the internal buffer is not empty and `is_eof` is false.
    ///   On eof, it will remain empty.
    ///
    /// This method is a poll method instead of async method since ssh channel
    /// alsoo needs to take care of the "windows" size, which is shared between
    /// normal data packet (stdout or rx part of forwarding) and
    /// the extended_data packet (stderr).
    ///
    /// If poll_for_data is called again before previously registered waker
    /// get awakened, then the previously stored waker will be replaced
    /// with the new waker.
    pub(super) fn poll_for_data(
        &self,
        cx: &mut Context<'_>,
        alt_buffer: &mut Vec<Bytes>,
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
impl SpscBytesChannel {
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
