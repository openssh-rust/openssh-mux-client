use std::sync::{atomic::AtomicU8, Arc};

use super::{ChannelDataArenaArc, SharedData};

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
// Use C repr so that we can decide order of fields here
// and avoid false sharing if possible.
#[repr(C)]
pub(super) struct ChannelData {
    pub(super) state: ChannelState,

    pub(super) pending_requests: PendingRequests,

    /// Number of receivers alive.
    /// Max value is 2, since there can only be rx (stdout)
    /// and stderr.
    pub(super) receivers_count: AtomicU8,

    /// Usually stdin for process or rx for forwarding.
    ///
    /// Put it in `Option<Arc<...>>` since it is optional
    /// and also avoid false sharing.
    ///
    /// Using Arc instead of Box here so that the reader
    /// can receive bytes out of it without `unwrap`.
    pub(super) rx: Option<Arc<MpscBytesChannel>>,

    /// Usually stderr for process
    ///
    /// Put it in `Option<Arc<...>>` since it is optional
    /// and also avoid false sharing.
    ///
    /// Using Arc instead of Box here so that the reader
    /// can receive bytes out of it without `unwrap`.
    pub(super) stderr: Option<Arc<MpscBytesChannel>>,

    /// Use u64 to avoid overflow.
    pub(super) sender_window_size: AwaitableAtomicU64,
}

/// Reference to the channel.
/// Would send close on drop.
#[derive(Clone, Debug)]
struct ChannelRef {
    shared_data: SharedData,
    channel_data: ChannelDataArenaArc,
}

impl Drop for ChannelRef {
    fn drop(&mut self) {
        // Send close
        todo!()
    }
}
