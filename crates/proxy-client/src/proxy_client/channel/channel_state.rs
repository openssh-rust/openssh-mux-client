use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

use strum::IntoStaticStr;

use crate::{
    error::{Error, OpenFailure},
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
#[derive(Debug, IntoStaticStr)]
enum State {
    /// Sent open channel request
    OpenChannelRequested(OpenChannelRequestedInner),

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

#[derive(Debug)]
pub(super) enum ProcessStatus {
    ProcessExited(ExitStatus),
    ProcessKilled(ExitSignal),
}

#[derive(Copy, Clone, Debug)]
pub(super) struct OpenChannelRequestedInner {
    pub(super) init_receiver_win_size: u32,

    /// The packet to sent to expend window size.
    /// It should have all the data required.
    ///
    /// We use an array here instead of `Bytes` here since the data
    /// will be stored for the entire channel.
    ///
    /// As such, the underlying heap allocation used by `Bytes` cannot be
    /// freed or reuse until the channel is closed.
    ///
    /// That is going to waste a lot of memory and have fragmentation.
    ///
    /// Thus, what we do here is to store an array instead and copy it
    /// into a `BytesMut` and then `.split().freeze()` it on demands
    /// to reduce fragmentation.
    pub(super) extend_window_size_packet: [u8; 14],
}

/// For the channel users
impl ChannelState {
    /// * `extend_window_size_packet` - The packet to sent to expend window size.
    ///   It should have all the data required.
    pub(super) fn new(init_receiver_win_size: u32, extend_window_size_packet: [u8; 14]) -> Self {
        Self(Mutex::new(Inner {
            state: State::OpenChannelRequested(OpenChannelRequestedInner {
                init_receiver_win_size,
                extend_window_size_packet,
            }),
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

                        // Release lock
                        drop(guard);

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

    /// Must be called after `wait_for_confirmation` returns
    /// `OpenChennelRes::Confirmed`
    pub(super) fn wait_for_process_exit(&self) -> impl Future<Output = ProcessStatus> + '_ {
        struct WaitForProcessExit<'a>(&'a ChannelState);

        impl Future for WaitForProcessExit<'_> {
            type Output = ProcessStatus;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match guard.state {
                    State::OpenChannelRequestConfirmed { .. } => {
                        let prev_waker = mem::replace(&mut guard.waker, Some(cx.waker().clone()));

                        // Release lock
                        drop(guard);

                        drop(prev_waker);

                        Poll::Pending
                    }
                    State::ProcessKilled(..) | State::ProcessExited(..) => {
                        let prev_state = mem::replace(&mut guard.state, State::Consumed);

                        // Release lock
                        drop(guard);

                        Poll::Ready(match prev_state {
                            State::ProcessExited(exit_status) => {
                                ProcessStatus::ProcessExited(exit_status)
                            }
                            State::ProcessKilled(exit_signal) => {
                                ProcessStatus::ProcessKilled(exit_signal)
                            }
                            _ => unreachable!(),
                        })
                    }
                    _ => panic!("Unexpected state"),
                }
            }
        }

        WaitForProcessExit(self)
    }
}

/// For the channel read task.
impl ChannelState {
    /// Must be only called once by the channel read task.
    pub(super) fn set_channel_open_res(
        &self,
        res: OpenChennelRes,
    ) -> Result<OpenChannelRequestedInner, Error> {
        let mut guard = self.0.lock().unwrap();

        if let State::OpenChannelRequested(inner) = guard.state {
            guard.state = match res {
                OpenChennelRes::Confirmed {
                    max_sender_packet_size,
                } => State::OpenChannelRequestConfirmed {
                    max_sender_packet_size,
                },
                OpenChennelRes::Failed(err) => State::OpenChannelRequestFailed(err),
            };

            Self::wakeup(guard);

            Ok(inner)
        } else {
            Err(Error::UnexpectedChannelState {
                expected_state: &"OpenChannelRequested",
                actual_state: (&guard.state).into(),
                msg: &"Received open channel request response",
            })
        }
    }

    /// Must be called after `set_channel_open_res`.
    pub(super) fn set_channel_process_status(&self, status: ProcessStatus) -> Result<(), Error> {
        let mut guard = self.0.lock().unwrap();

        if let State::OpenChannelRequestConfirmed { .. } = guard.state {
            guard.state = match status {
                ProcessStatus::ProcessExited(exit_status) => State::ProcessExited(exit_status),
                ProcessStatus::ProcessKilled(exit_signal) => State::ProcessKilled(exit_signal),
            };

            Self::wakeup(guard);

            Ok(())
        } else {
            Err(Error::UnexpectedChannelState {
                expected_state: &"OpenChannelRequestConfirmed",
                actual_state: (&guard.state).into(),
                msg: &"Received process exit status",
            })
        }
    }

    fn wakeup(mut guard: MutexGuard<'_, Inner>) {
        let waker = guard.waker.take();

        // Release lock
        drop(guard);

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
