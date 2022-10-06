use std::sync::{atomic::AtomicU8, Arc};

use bytes::BytesMut;

use super::{ChannelDataArenaArc, SharedData};
use crate::request::ChannelClose;

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

mod channel_input;
pub use channel_input::ChannelInput;

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

    /// Usually stdout for process or rx for forwarding.
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
#[derive(Debug)]
struct ChannelRef {
    shared_data: SharedData,
    channel_data: ChannelDataArenaArc,
    buffer: BytesMut,
}

impl Clone for ChannelRef {
    fn clone(&self) -> Self {
        Self {
            shared_data: self.shared_data.clone(),
            channel_data: self.channel_data.clone(),
            buffer: BytesMut::new(),
        }
    }
}

impl ChannelRef {
    fn channel_id(&self) -> u32 {
        ChannelDataArenaArc::slot(&self.channel_data)
    }
}

impl Drop for ChannelRef {
    fn drop(&mut self) {
        // Send close
        todo!()
    }
}
