use std::{
    io,
    ops::Deref,
    pin::Pin,
    sync::{atomic::Ordering::Relaxed, Arc},
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{
    ready,
    stream::{FusedStream, Stream},
};
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};

use super::{ChannelRef, MpscBytesChannel};

#[derive(Debug)]
pub struct ChannelOutput {
    channel_ref: ChannelRef,

    channel: Arc<MpscBytesChannel>,

    /// FIFO List of bytes.
    /// The queue head is at the end of the vec.
    /// Every `Bytes` in it must not be empty.
    fifo: Vec<Bytes>,

    is_eof: bool,
}

impl ChannelOutput {
    /// If self.fifo is not empty, ret.
    /// Otherwise poll for data.
    fn poll_for_data(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = Pin::into_inner(self);

        if !this.is_eof && this.fifo.is_empty() {
            let fifo = &mut this.fifo;

            // If Poll::Pending is returned, then nothing has changed.
            // Otherwise, fifo either contains new data, or is empty
            // due to eof.
            ready!(this.channel.poll_for_data(fifo, cx));

            fifo.reverse();

            this.is_eof = fifo.is_empty();
        }

        Poll::Ready(())
    }
}

impl Stream for ChannelOutput {
    type Item = Bytes;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        ready!(self.as_mut().poll_for_data(cx));

        // If self.is_eof == true, then self.fifo.pop() would return None.
        // Otherwise, it would return Some.
        Poll::Ready(self.fifo.pop())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.fifo.len(), self.is_eof.then_some(0))
    }
}

impl FusedStream for ChannelOutput {
    fn is_terminated(&self) -> bool {
        self.is_eof
    }
}

impl AsyncBufRead for ChannelOutput {
    fn poll_fill_buf(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        ready!(self.as_mut().poll_for_data(cx));

        Poll::Ready(Ok(Pin::into_inner(self)
            .fifo
            .last()
            .map(Deref::deref)
            .unwrap_or(&[])))
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        let fifo = &mut self.fifo;

        if amt == 0 {
            return;
        }

        let err_msg = "amt is larger than number of bytes returned in <ChannelOutput as AsyncBufRead>::poll_fill_buf";

        let bytes = fifo.last_mut().expect(err_msg);

        assert!(
            amt <= bytes.len(),
            "{err_msg}: amt = {amt} > {}",
            bytes.len()
        );

        if bytes.len() == amt {
            fifo.pop().expect(err_msg);
        } else {
            // Afterwards, bytes contains [amt, len).
            let _: Bytes = bytes.split_to(amt);
        }
    }
}
impl AsyncRead for ChannelOutput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let remaining = buf.remaining();
            if remaining == 0 {
                break Poll::Ready(Ok(()));
            }

            let slice = ready!(self.as_mut().poll_fill_buf(cx))?;
            if slice.is_empty() {
                break Poll::Ready(Ok(()));
            }

            // n must not be 0 since remaining != 0 and slice.len() != 0
            let n = remaining.min(slice.len());

            buf.put_slice(&slice[..n]);
            self.as_mut().consume(n);
        }
    }
}

impl Drop for ChannelOutput {
    fn drop(&mut self) {
        // Decrease receivers_count.
        //
        // Once it is reduced to 0, the channel would not
        // send any new extend win request anymore.
        let prev_cnt = self
            .channel_ref
            .channel_data
            .receivers_count
            .fetch_sub(1, Relaxed);

        // If prev_cnt, then the fetch_sub operatio underflows.
        debug_assert_ne!(prev_cnt, 0);

        // Drop the reader, any write to it will be ignored
        // and its internal buffer/waker dropped.
        self.channel.drop_reader();
    }
}
