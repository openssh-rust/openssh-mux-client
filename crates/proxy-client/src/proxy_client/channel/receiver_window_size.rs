use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

use bytes::Bytes;

/// Help maintain receiver window size so that the data/extended_data channel
/// would not block forever.
#[derive(Debug)]
pub(super) struct ReceiverWindowSize {
    /// The packet to sent to expend window size.
    /// It should have all the data required.
    extend_window_size_packet: Bytes,

    initial_window_size: u32,

    /// Number of `SpscBytesChannel` that is not closed
    opened_spsc_bytes_channel_count: AtomicU8,
}

impl ReceiverWindowSize {
    pub(super) fn new(
        extend_window_size_packet: Bytes,
        initial_window_size: u32,
        opened_spsc_bytes_channel_count: u8,
    ) -> Self {
        Self {
            extend_window_size_packet,
            initial_window_size,
            opened_spsc_bytes_channel_count: AtomicU8::new(opened_spsc_bytes_channel_count),
        }
    }

    /// Decrease `opened_spsc_bytes_channel_count` by one.
    pub(super) fn decr_opened_spsc_bytes_channel_count(&self) {
        self.opened_spsc_bytes_channel_count.fetch_sub(1, Relaxed);
    }

    pub(super) fn get_opened_spsc_bytes_channel_count(&self) -> u8 {
        self.opened_spsc_bytes_channel_count.load(Relaxed)
    }
}
