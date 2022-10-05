use std::{
    convert::TryInto,
    num::{NonZeroU32, NonZeroU64},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{ready, sink::Sink};

use super::ChannelRef;
use crate::{request::DataTransfer, Error};

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

    buffer: BytesMut,
}

impl ChannelInput {
    fn add_pending_byte(&mut self, bytes: Bytes) {
        self.pending_bytes.push(bytes);
    }

    /// * `n` - number of bytes to write
    ///
    /// This function would not modify any existing data in `self.buffer`
    fn create_data_transfer_header(&mut self, n: u32) -> Result<Bytes, Error> {
        DataTransfer::create_header(self.channel_ref.channel_id(), n, &mut self.buffer)
    }

    fn try_flush(&mut self) -> Result<(), Error> {
        // Maximum number of bytes we can write to
        let max = self
            .max_packet_size
            .get()
            .min(self.curr_sender_win.try_into().unwrap_or(u32::MAX));

        if max == 0 {
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

        self.channel_ref
            .shared_data
            .get_write_channel()
            .add_more_data(|buffer| {
                buffer.push(header);

                // Move the bytes into buffer;
                buffer.extend(pending_bytes.drain(0..pending_end));

                if let Some(bytes) = pending_bytes.first_mut() {
                    let n = bytes.len().min(max);

                    if n != 0 {
                        // bytes.split_to(n) returns Bytes containing bytes[0, n),
                        // and afterwards bytes contains [n, len)
                        buffer.push(bytes.split_to(n));
                    }

                    bytes_written += n;
                }
            });

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
        }

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        while !self.pending_bytes.is_empty() {
            if self.curr_sender_win == 0 {
                ready!(self.as_mut().poll_ready(cx))?;
            }

            Pin::into_inner(self.as_mut()).try_flush()?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}

impl Drop for ChannelInput {
    fn drop(&mut self) {
        // Send Eof
        todo!()
    }
}
