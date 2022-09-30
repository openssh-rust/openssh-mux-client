use std::{
    mem,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

use bytes::Bytes;

use crate::{
    error::OpenFailure,
    response::{ExitSignal, ExitStatus},
};

#[derive(Debug)]
pub(super) struct ChannelState(Mutex<Inner>);

#[derive(Debug)]
struct Inner {
    state: State,
    waker: Option<Waker>,
}

#[derive(Debug)]
enum State {
    /// Sent open channel request
    OpenChannelRequested {
        init_receiver_win_size: u32,

        /// The packet to sent to expend window size.
        /// It should have all the data required.
        extend_window_size_packet: Bytes,
    },

    OpenChannelRequestConfirmed {
        max_sender_packet_size: u32,
    },

    // Terminating states
    OpenChannelRequestFailed(OpenFailure),

    ProcessExited(ExitStatus),

    ProcessKilled(ExitSignal),
}
