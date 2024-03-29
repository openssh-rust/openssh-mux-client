use std::{
    convert::TryInto,
    io, mem,
    num::{NonZeroU32, NonZeroU64},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{ready, Sink, SinkExt};
use pin_project::{pin_project, pinned_drop};
use tokio::io::AsyncWrite;
use tokio_util::sync::WaitForCancellationFutureOwned;

use super::ChannelRef;
use crate::{
    request::{ChannelEof, DataTransfer},
    Error,
};

/// Input of the Channel
#[derive(Debug)]
#[pin_project(PinnedDrop)]
pub struct ChannelInput {
    channel_ref: ChannelRef,

    max_packet_size: NonZeroU32,

    /// Number of bytes one can send
    /// without waiting.
    curr_sender_win: u64,

    /// Bytes that haven't been sent yet.
    pending_bytes: Vec<Bytes>,
    pending_len: usize,

    buffer: BytesMut,

    #[pin]
    token: WaitForCancellationFutureOwned,
}

impl ChannelInput {
    fn add_pending_byte(self: Pin<&mut Self>, bytes: Bytes) {
        let this = self.project();

        *this.pending_len += bytes.len();
        this.pending_bytes.push(bytes);
    }

    fn update_curr_sender_win_size(self: Pin<&mut Self>) {
        let this = self.project();

        *this.curr_sender_win += this.channel_ref.channel_data.sender_window_size.get();
    }

    /// * `n` - number of bytes to write
    ///
    /// This function would not modify any existing data in `self.buffer`
    fn create_data_transfer_header(self: Pin<&mut Self>, n: u32) -> Result<Bytes, Error> {
        let this = self.project();

        let channel_id = this.channel_ref.channel_id();

        let buffer = this.buffer;

        let before = buffer.len();
        let res = DataTransfer::create_header(channel_id, n, buffer);
        let after = buffer.len();

        debug_assert_eq!(before, after);

        res
    }

    fn try_flush(mut self: Pin<&mut Self>) -> Result<(), Error> {
        let this = self.as_mut().project();

        // Maximum number of bytes we can write to
        let max = this
            .max_packet_size
            .get()
            .min((*this.curr_sender_win).try_into().unwrap_or(u32::MAX));

        if max == 0 || this.pending_bytes.is_empty() {
            return Ok(());
        }

        let header = self.as_mut().create_data_transfer_header(max)?;

        let this = self.as_mut().project();

        let pending_bytes = this.pending_bytes;

        let mut max: usize = max.try_into().unwrap_or(usize::MAX);
        let mut bytes_written: usize = 0;

        // Calculate number of bytes that can be directly
        // moved into the buffer.
        //
        // This would also decrement n until it is smaller than
        // the the first `Bytes` that cannot be directly moved into
        // the buffer, which means only part of that `Bytes` can
        // be written into the buffer.
        let pending_end = pending_bytes
            .iter()
            .take_while(|bytes| {
                let take = max >= bytes.len();

                if take {
                    max -= bytes.len();
                    bytes_written += bytes.len();
                }

                take
            })
            .count();

        // If max != 0 and pending_bytes.len() > pending_end,
        // then calculate the last bytes to add.
        let mut maybe_last_bytes = None;

        if let Some(bytes) = pending_bytes.get_mut(pending_end) {
            let n = bytes.len().min(max);

            if n != 0 {
                // bytes.split_to(n) returns Bytes containing bytes[0, n),
                // and afterwards bytes contains [n, len)
                maybe_last_bytes = Some(bytes.split_to(n));
            }

            bytes_written += n;
        }

        let mut drain = pending_bytes.drain(0..pending_end);

        this.channel_ref
            .shared_data
            .get_write_channel()
            .add_more_data(
                1 + drain.len() + maybe_last_bytes.iter().len(),
                Some(header)
                    .into_iter()
                    // Use mutable alias to drain since add_more_data internally
                    // holds a mutex, so here we drop `drain` outside of it to
                    // reduce critical section.
                    .chain(&mut drain)
                    .chain(maybe_last_bytes),
            );

        *this.pending_len -= bytes_written;

        let bytes_written: u64 = bytes_written.try_into().unwrap();
        *this.curr_sender_win -= bytes_written;

        Ok(())
    }
}

impl Sink<Bytes> for ChannelInput {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();

        if *this.curr_sender_win == 0 {
            *this.curr_sender_win = ready!(this
                .channel_ref
                .channel_data
                .sender_window_size
                .poll_until_non_zero(cx)
                .map(NonZeroU64::get));
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, bytes: Bytes) -> Result<(), Self::Error> {
        if !bytes.is_empty() {
            self.as_mut().add_pending_byte(bytes);

            self.as_mut().update_curr_sender_win_size();

            let this = self.as_mut().project();

            let curr_sender_win: usize = (*this.curr_sender_win).try_into().unwrap_or(usize::MAX);
            let max_packet_size: usize =
                this.max_packet_size.get().try_into().unwrap_or(usize::MAX);

            if curr_sender_win > 0 && self.pending_len >= curr_sender_win.min(max_packet_size) {
                self.try_flush()?;
            }
        }

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        while !self.as_mut().project().pending_bytes.is_empty() {
            if *self.as_mut().project().curr_sender_win == 0 {
                ready!(self.as_mut().poll_ready(cx))?;
            } else {
                // Try to send as much as we can in one single packet
                self.as_mut().update_curr_sender_win_size();
            }

            self.as_mut().try_flush()?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::poll_flush(self, cx)
    }
}

impl AsyncWrite for ChannelInput {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let this = self.as_mut().project();

        let buffer = this.buffer;

        debug_assert!(buffer.is_empty());
        buffer.clear();

        buffer.extend_from_slice(buf);
        let bytes = buffer.split().freeze();
        let len = bytes.len();

        self.start_send(bytes)?;

        Poll::Ready(Ok(len))
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        if bufs.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let this = self.as_mut().project();

        let buffer = this.buffer;

        debug_assert!(buffer.is_empty());
        buffer.clear();

        let len: usize = bufs.iter().map(|io_slice| io_slice.len()).sum();

        buffer.reserve(len);

        for buf in bufs {
            buffer.extend_from_slice(buf);
        }

        let bytes = buffer.split().freeze();

        self.start_send(bytes)?;

        Poll::Ready(Ok(len))
    }
    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Sink::poll_flush(self, cx).map_err(io::Error::from)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Sink::poll_close(self, cx).map_err(io::Error::from)
    }
}

impl ChannelInput {
    fn send_eof_packet(self: Pin<&mut Self>) {
        let this = self.project();

        let channel_id = this.channel_ref.channel_id();

        let buffer = this.buffer;
        debug_assert!(buffer.is_empty());
        buffer.clear();

        ChannelEof::new(channel_id)
            .serialize_with_header(buffer, 0)
            .expect("Serialization should not fail here");
        let bytes = buffer.split().freeze();

        this.channel_ref
            .shared_data
            .get_write_channel()
            .push_bytes(bytes);
    }
}

#[pinned_drop]
impl PinnedDrop for ChannelInput {
    fn drop(mut self: Pin<&mut Self>) {
        if self.as_mut().project().pending_bytes.is_empty() {
            self.send_eof_packet();
        } else {
            self.as_mut().update_curr_sender_win_size();

            if self.as_mut().try_flush().is_err()
                || self.as_mut().project().pending_bytes.is_empty()
            {
                self.send_eof_packet();
            } else {
                let this = self.project();

                // Send all pending data in another task
                //
                // After constructing `new_channel_input`,
                // the old one does not contain any pending data at all,
                // and it would be simply dropped without sending eof.
                let new_channel_input = ChannelInput {
                    channel_ref: this.channel_ref.clone(),
                    max_packet_size: *this.max_packet_size,
                    curr_sender_win: *this.curr_sender_win,

                    pending_bytes: mem::take(this.pending_bytes),
                    pending_len: mem::take(this.pending_len),

                    buffer: mem::take(this.buffer),

                    token: this
                        .channel_ref
                        .shared_data
                        .get_cancellation_token()
                        .clone()
                        .cancelled_owned(),
                };
                tokio::spawn(async move {
                    tokio::pin!(new_channel_input);
                    if new_channel_input.as_mut().close().await.is_err() {
                        // Make sure drop implementation would send eof packet
                        // instead of trying to flush the data again
                        // or create yet another task.
                        new_channel_input.project().pending_bytes.clear();
                    }
                });
            }
        }
    }
}
