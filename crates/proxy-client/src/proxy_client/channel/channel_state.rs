use std::{
    future::Future,
    mem,
    pin::Pin,
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

/// Expected state transition:
///
/// OpenChannelRequested => OpenChannelRequestConfirmed => ProcessExited | ProcessKilled => Consumed
///
/// or
///
/// OpenChannelRequested => OpenChannelRequestFailed => Consumed
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

    OpenChannelRequestFailed(OpenFailure),

    ProcessExited(ExitStatus),

    ProcessKilled(ExitSignal),

    Consumed,
}

#[derive(Debug)]
pub(super) enum OpenChennelRes {
    /// Ok and confirmed
    Confirmed {
        max_sender_packet_size: u32,
    },
    Failed(OpenFailure),
}

impl ChannelState {
    /// * `extend_window_size_packet` - The packet to sent to expend window size.
    ///   It should have all the data required.
    pub(super) fn new(init_receiver_win_size: u32, extend_window_size_packet: Bytes) -> Self {
        Self(Mutex::new(Inner {
            state: State::OpenChannelRequested {
                init_receiver_win_size,
                extend_window_size_packet,
            },
            waker: None,
        }))
    }

    pub(super) fn wait_for_confirmation(&self) -> impl Future<Output = OpenChennelRes> + '_ {
        struct WaitForConfirmation<'a>(&'a ChannelState);

        impl Future for WaitForConfirmation<'_> {
            type Output = OpenChennelRes;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match guard.state {
                    State::OpenChannelRequested { .. } => {
                        let prev_waker = mem::replace(&mut guard.waker, Some(cx.waker().clone()));

                        // Release lock
                        drop(guard);

                        drop(prev_waker);

                        Poll::Pending
                    }
                    State::OpenChannelRequestConfirmed {
                        max_sender_packet_size,
                    } => Poll::Ready(OpenChennelRes::Confirmed {
                        max_sender_packet_size,
                    }),
                    State::OpenChannelRequestFailed(..) => {
                        let prev_state = mem::replace(&mut guard.state, State::Consumed);

                        if let State::OpenChannelRequestFailed(err) = prev_state {
                            Poll::Ready(OpenChennelRes::Failed(err))
                        } else {
                            unreachable!()
                        }
                    }
                    _ => panic!("Unexpected state"),
                }
            }
        }

        WaitForConfirmation(self)
    }
}
