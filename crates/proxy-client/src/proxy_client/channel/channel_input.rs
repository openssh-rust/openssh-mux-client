use std::{
    convert::TryInto,
    io, mem,
    num::{NonZeroU32, NonZeroU64},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{ready, Sink, SinkExt};
use tokio::io::AsyncWrite;

use super::ChannelRef;
use crate::{
    request::{ChannelEof, DataTransfer},
    Error,
};

/// Input of the Channel
#[derive(Debug)]
pub struct ChannelInput {
    channel_ref: ChannelRef,

    max_packet_size: NonZeroU32,

    /// Number of bytes one can send
    /// without waiting.
    curr_sender_win: u64,

    /// Bytes that haven't been sent yet.
    pending_bytes: Vec<Bytes>,
    pending_len: usize,
}

impl ChannelInput {
    fn add_pending_byte(&mut self, bytes: Bytes) {
        self.pending_len += bytes.len();
        self.pending_bytes.push(bytes);
    }

    fn update_curr_sender_win_size(&mut self) {
        self.curr_sender_win += self.channel_ref.channel_data.sender_window_size.get();
    }

    /// * `n` - number of bytes to write
    ///
    /// This function would not modify any existing data in `self.buffer`
    fn create_data_transfer_header(&mut self, n: u32) -> Result<Bytes, Error> {
        let channel_id = self.channel_ref.channel_id();

        let buffer = &mut self.channel_ref.buffer;

        let before = buffer.len();
        let res = DataTransfer::create_header(channel_id, n, buffer);
        let after = buffer.len();

        debug_assert_eq!(before, after);

        res
    }

    fn try_flush(&mut self) -> Result<(), Error> {
        // Maximum number of bytes we can write to
        let max = self
            .max_packet_size
            .get()
            .min(self.curr_sender_win.try_into().unwrap_or(u32::MAX));

        if max == 0 || self.pending_bytes.is_empty() {
            return Ok(());
        }

        let header = self.create_data_transfer_header(max)?;

        let pending_bytes = &mut self.pending_bytes;

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

        // The closure holds a mutex, put drain outside
        // so that the dropping of it is not included in the
        // critical section.
        let mut drain = pending_bytes.drain(0..pending_end);

        self.channel_ref
            .shared_data
            .get_write_channel()
            .add_more_data(|buffer| {
                buffer.reserve(1 + drain.len() + maybe_last_bytes.iter().len());

                buffer.push(header);

                // Move the bytes into buffer;
                buffer.extend(&mut drain);

                buffer.extend(maybe_last_bytes);
            });

        self.pending_len -= bytes_written;

        let bytes_written: u64 = bytes_written.try_into().unwrap();
        self.curr_sender_win -= bytes_written;

        Ok(())
    }
}

impl Sink<Bytes> for ChannelInput {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.curr_sender_win == 0 {
            self.curr_sender_win = ready!(self
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
            self.add_pending_byte(bytes);

            self.update_curr_sender_win_size();

            let curr_sender_win: usize = self.curr_sender_win.try_into().unwrap_or(usize::MAX);
            let max_packet_size: usize =
                self.max_packet_size.get().try_into().unwrap_or(usize::MAX);

            if curr_sender_win > 0 && self.pending_len >= curr_sender_win.min(max_packet_size) {
                Pin::into_inner(self.as_mut()).try_flush()?;
            }
        }

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        while !self.pending_bytes.is_empty() {
            if self.curr_sender_win == 0 {
                ready!(self.as_mut().poll_ready(cx))?;
            } else {
                // Try to send as much as we can in one single packet
                self.update_curr_sender_win_size();
            }

            Pin::into_inner(self.as_mut()).try_flush()?;
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

        let buffer = &mut self.channel_ref.buffer;

        debug_assert!(buffer.is_empty());
        buffer.clear();

        buffer.extend_from_slice(buf);
        let bytes = buffer.split().freeze();
        let len = bytes.len();

        self.start_send(bytes).map_err(Error::into_io_error)?;

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

        let buffer = &mut self.channel_ref.buffer;

        debug_assert!(buffer.is_empty());
        buffer.clear();

        let len: usize = bufs.iter().map(|io_slice| io_slice.len()).sum();

        buffer.reserve(len);

        for buf in bufs {
            buffer.extend_from_slice(buf);
        }

        let bytes = buffer.split().freeze();

        self.start_send(bytes).map_err(Error::into_io_error)?;

        Poll::Ready(Ok(len))
    }
    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Sink::poll_flush(self, cx).map_err(Error::into_io_error)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Sink::poll_close(self, cx).map_err(Error::into_io_error)
    }
}

impl ChannelInput {
    fn send_eof_packet(&mut self) -> Result<(), Error> {
        let channel_id = self.channel_ref.channel_id();

        let buffer = &mut self.channel_ref.buffer;
        debug_assert!(buffer.is_empty());
        buffer.clear();

        ChannelEof::new(channel_id).serialize_with_header(buffer, 0)?;
        let bytes = buffer.split().freeze();

        self.channel_ref
            .shared_data
            .get_write_channel()
            .push_bytes(bytes);

        Ok(())
    }
}

impl Drop for ChannelInput {
    fn drop(&mut self) {
        if self.pending_bytes.is_empty() {
            self.send_eof_packet().ok();
        } else {
            self.update_curr_sender_win_size();

            if self.try_flush().is_err() || self.pending_bytes.is_empty() {
                self.send_eof_packet().ok();
            } else {
                // Send all pending data in another task
                //
                // After constructing `new_channel_input`,
                // the old one does not contain any pending data at all,
                // and it would be simply dropped without sending eof.
                let mut new_channel_input = ChannelInput {
                    channel_ref: self.channel_ref.clone(),
                    max_packet_size: self.max_packet_size,
                    curr_sender_win: self.curr_sender_win,

                    pending_bytes: mem::take(&mut self.pending_bytes),
                    pending_len: mem::take(&mut self.pending_len),
                };
                tokio::spawn(async move {
                    if new_channel_input.close().await.is_err() {
                        // Make sure drop implementation would send eof packet
                        // instead of trying to flush the data again
                        // or create yet another task.
                        new_channel_input.pending_bytes.clear();
                    }
                });
            }
        }
    }
}
