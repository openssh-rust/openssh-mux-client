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
pub(crate) struct ChannelState(Mutex<Inner>);

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
        max_packet_size: u32,
    },

    OpenChannelRequestFailed(OpenFailure),

    ProcessExited(ExitStatus),

    ProcessKilled(ExitSignal),

    Consumed,
}

#[derive(Debug)]
pub(crate) enum OpenChannelRes {
    /// Ok and confirmed
    Confirmed {
        max_packet_size: u32,
    },
    Failed(OpenFailure),
}

#[derive(Debug)]
pub(crate) enum ProcessStatus {
    ProcessExited(ExitStatus),
    ProcessKilled(ExitSignal),
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct OpenChannelRequestedInner {
    pub(crate) init_receiver_win_size: u32,

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
    pub(crate) extend_window_size_packet: [u8; 14],

    /// The number of bytes `extend_window_size_packet` will extend
    /// the receiver window size.
    pub(crate) extend_window_size: u32,
}

/// For the channel users
impl ChannelState {
    /// * `extend_window_size_packet` - The packet to sent to expend window size.
    ///   It should have all the data required.
    pub(crate) fn new(
        init_receiver_win_size: u32,
        extend_window_size_packet: [u8; 14],
        extend_window_size: u32,
    ) -> Self {
        Self(Mutex::new(Inner {
            state: State::OpenChannelRequested(OpenChannelRequestedInner {
                init_receiver_win_size,
                extend_window_size_packet,
                extend_window_size,
            }),
            waker: None,
        }))
    }

    pub(crate) fn wait_for_confirmation(&self) -> impl Future<Output = OpenChannelRes> + '_ {
        struct WaitForConfirmation<'a>(&'a ChannelState);

        impl Future for WaitForConfirmation<'_> {
            type Output = OpenChannelRes;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match guard.state {
                    State::OpenChannelRequested { .. } => {
                        ChannelState::install_new_waker(guard, cx);

                        Poll::Pending
                    }
                    State::OpenChannelRequestConfirmed { max_packet_size } => {
                        Poll::Ready(OpenChannelRes::Confirmed { max_packet_size })
                    }
                    State::OpenChannelRequestFailed(..) => {
                        let prev_state = mem::replace(&mut guard.state, State::Consumed);

                        // Release lock
                        drop(guard);

                        if let State::OpenChannelRequestFailed(err) = prev_state {
                            Poll::Ready(OpenChannelRes::Failed(err))
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
    /// `OpenChannelRes::Confirmed`
    pub(crate) fn wait_for_process_exit(&self) -> impl Future<Output = ProcessStatus> + '_ {
        struct WaitForProcessExit<'a>(&'a ChannelState);

        impl Future for WaitForProcessExit<'_> {
            type Output = ProcessStatus;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match guard.state {
                    State::OpenChannelRequestConfirmed { .. } => {
                        ChannelState::install_new_waker(guard, cx);

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

    fn install_new_waker(mut guard: MutexGuard<'_, Inner>, cx: &mut Context<'_>) {
        let prev_waker = mem::replace(&mut guard.waker, Some(cx.waker().clone()));

        // Release lock
        drop(guard);

        drop(prev_waker);
    }
}

/// For the channel read task.
impl ChannelState {
    /// Must be only called once by the channel read task.
    pub(crate) fn set_channel_open_res(
        &self,
        res: OpenChannelRes,
    ) -> Result<OpenChannelRequestedInner, Error> {
        let mut guard = self.0.lock().unwrap();

        if let State::OpenChannelRequested(inner) = guard.state {
            guard.state = match res {
                OpenChannelRes::Confirmed { max_packet_size } => {
                    State::OpenChannelRequestConfirmed { max_packet_size }
                }
                OpenChannelRes::Failed(err) => State::OpenChannelRequestFailed(err),
            };

            Self::wakeup(guard);

            Ok(inner)
        } else {
            Err(Error::UnexpectedChannelState {
                expected_state: &"OpenChannelRequested",
                actual_state: (&guard.state).into(),
            })
        }
    }

    /// Must be called after `set_channel_open_res`.
    pub(crate) fn set_channel_process_status(&self, status: ProcessStatus) -> Result<(), Error> {
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
