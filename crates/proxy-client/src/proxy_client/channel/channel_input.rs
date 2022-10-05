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
    /// Cummutative len of all pending_bytes
    pending_len: usize,

    buffer: BytesMut,
}

impl ChannelInput {
    fn add_pending_byte(&mut self, bytes: Bytes) {
        self.pending_len += bytes.len();
        self.pending_bytes.push(bytes);
    }

    /// * `n` - number of bytes to write
    ///
    /// This function would not modify any existing data in `self.buffer`
    fn create_data_transfer_header(&mut self, n: u32) -> Result<Bytes, Error> {
        DataTransfer::create_header(self.channel_ref.channel_id(), n, &mut self.buffer)
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
        self.add_pending_byte(bytes);

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.curr_sender_win == 0 {
            ready!(self.as_mut().poll_ready(cx))?;
        }

        let n = self
            .max_packet_size
            .get()
            .min(self.curr_sender_win.try_into().unwrap_or(u32::MAX))
            .min(self.pending_len.try_into().unwrap_or(u32::MAX));

        if n == 0 {
            return Poll::Ready(Ok(()));
        }

        let header = self.create_data_transfer_header(n);

        todo!()
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
