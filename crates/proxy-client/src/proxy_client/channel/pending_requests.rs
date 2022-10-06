use std::{
    future::Future,
    mem,
    num::NonZeroUsize,
    pin::Pin,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

/// Suitable for spsc.
///
/// Send one or multiple requests and use
/// [`PendingRequests`] to wait on all of them.
#[derive(Debug, Default)]
pub(crate) struct PendingRequests(Mutex<Inner>);

#[derive(Debug, Default)]
enum Inner {
    #[default]
    NotStarted,

    Waiting {
        /// usize is enough since all requests have to be buffered in memory
        /// before sending.
        pending_requests: NonZeroUsize,
        waker: Option<Waker>,
    },

    Done(Completion),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum Completion {
    /// All requests succeeded
    Success,
    /// Some requests failed
    Failed,
}

impl PendingRequests {
    /// This function must be called before new requests
    /// are flushed.
    ///
    /// Once start_new_requests, wait_for_completion must be called.
    pub(crate) async fn start_new_requests<F>(&self, requests: NonZeroUsize) {
        struct WaitForPrevCompletion<'a>(&'a PendingRequests);

        impl<'a> Future for WaitForPrevCompletion<'a> {
            type Output = MutexGuard<'a, Inner>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match &mut *guard {
                    Inner::Done(..) | Inner::NotStarted => Poll::Ready(guard),
                    Inner::Waiting { waker, .. } => {
                        let prev_waker = mem::replace(waker, Some(cx.waker().clone()));

                        // Release mutex
                        drop(guard);

                        drop(prev_waker);

                        Poll::Pending
                    }
                }
            }
        }

        let mut guard = WaitForPrevCompletion(self).await;

        debug_assert!(matches!(&*guard, Inner::NotStarted | Inner::Done { .. }));

        // This overwrites should be simply memcpy.
        // Dropping the old value should be zero-cost.
        *guard = Inner::Waiting {
            pending_requests: requests,
            waker: None,
        };
    }

    /// Must be called once after `start_new_requests` is called and
    /// data flushed.
    ///
    /// This function must be called after
    /// [`PendingRequests::start_new_requests`] is called.
    pub(crate) fn wait_for_completion(&self) -> impl Future<Output = Completion> + '_ {
        struct WaitForCompletion<'a>(&'a PendingRequests);

        impl Future for WaitForCompletion<'_> {
            type Output = Completion;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0 .0.lock().unwrap();

                match &mut *guard {
                    Inner::Done(completion) => Poll::Ready(*completion),
                    Inner::Waiting { waker, .. } => {
                        let prev_waker = mem::replace(waker, Some(cx.waker().clone()));

                        // Release mutex
                        drop(guard);

                        drop(prev_waker);

                        Poll::Pending
                    }
                    Inner::NotStarted => {
                        panic!("wait_for_completion must be called after start_new_requests!")
                    }
                }
            }
        }

        WaitForCompletion(self)
    }

    /// Retrieve number of pending requests.
    pub(crate) fn retrieve_pending_requests(&self) -> Option<NonZeroUsize> {
        if let Inner::Waiting {
            pending_requests, ..
        } = &*self.0.lock().unwrap()
        {
            Some(*pending_requests)
        } else {
            None
        }
    }

    /// Report completion of all requests.
    pub(crate) fn report_request_completion(&self, completion: Completion) {
        let prev_state = mem::replace(&mut *self.0.lock().unwrap(), Inner::Done(completion));

        match prev_state {
            Inner::Waiting { waker, .. } => {
                if let Some(waker) = waker {
                    waker.wake()
                }
            }
            _ => panic!("Invalid state, expected `Waiting`!"),
        }
    }
}
