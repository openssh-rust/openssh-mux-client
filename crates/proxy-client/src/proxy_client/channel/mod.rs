use std::sync::atomic::{AtomicU64, AtomicU8};

mod channel_state;
pub(super) use channel_state::{
    ChannelState, OpenChannelRequestedInner, OpenChennelRes, ProcessStatus,
};

mod mpsc_bytes_channel;
pub(super) use mpsc_bytes_channel::MpscBytesChannel;

mod pending_requests;
pub(super) use pending_requests::{Completion, PendingRequests};

#[derive(Debug)]
pub(super) struct Channel {
    pub(super) state: ChannelState,

    pub(super) pending_requests: PendingRequests,

    /// TODO: Make an awaitable type for this
    pub(super) sender_window_size: AtomicU64,

    /// Number of receivers alive
    pub(super) receivers_count: AtomicU8,

    /// Usually stdin for process or rx for forwarding.
    pub(super) rx: Option<Box<MpscBytesChannel>>,

    /// Usually stderr for process
    pub(super) stderr: Option<Box<MpscBytesChannel>>,
}
