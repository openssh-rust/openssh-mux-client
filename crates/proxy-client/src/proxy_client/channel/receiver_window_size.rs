use std::sync::atomic::AtomicU8;

/// Help maintain receiver window size so that the data/extended_data channel
/// would not block forever.
#[derive(Debug)]
pub(super) struct ReceiverWindowSize {
    /// The packet to sent to expend window size.
    /// It should have all the data required.
    extend_window_size_packet: [u8; 16],

    /// Number of `SpscBytesChannel` that is not closed
    opened_spsc_bytes_channel_count: AtomicU8,
}

impl ReceiverWindowSize {
    pub(super) fn new(
        extend_window_size_packet: [u8; 16],
        opened_spsc_bytes_channel_count: u8,
    ) -> Self {
        Self {
            extend_window_size_packet,
            opened_spsc_bytes_channel_count: AtomicU8::new(opened_spsc_bytes_channel_count),
        }
    }
}
