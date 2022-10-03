use std::sync::atomic::{AtomicU64, AtomicU8};

mod channel_state;
pub(super) use channel_state::{
    ChannelState, OpenChannelRequestedInner, OpenChennelRes, ProcessStatus,
};

mod mpsc_bytes_channel;
pub(super) use mpsc_bytes_channel::MpscBytesChannel;

mod pending_requests;
pub(super) use pending_requests::{Completion, PendingRequests};

mod awaitable_atomic_u64;
pub(super) use awaitable_atomic_u64::AwaitableAtomicU64;

#[derive(Debug)]
pub(super) struct Channel {
    pub(super) state: ChannelState,

    pub(super) pending_requests: PendingRequests,

    /// Use u64 to avoid overflow.
    pub(super) sender_window_size: AwaitableAtomicU64,

    /// Number of receivers alive
    pub(super) receivers_count: AtomicU8,

    /// Usually stdin for process or rx for forwarding.
    pub(super) rx: Option<Box<MpscBytesChannel>>,

    /// Usually stderr for process
    pub(super) stderr: Option<Box<MpscBytesChannel>>,
}
