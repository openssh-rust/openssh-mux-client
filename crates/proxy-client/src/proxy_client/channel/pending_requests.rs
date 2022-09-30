use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
        Mutex,
    },
    task::{Context, Poll, Waker},
};

/// Suitable for spsc.
///
/// Send one or multiple requests and use
/// [`PendingRequests`] to wait on all of them.
#[derive(Default, Debug)]
pub(super) struct PendingRequests {
    /// Number of requests that has not yet received responses.
    pending_requests: AtomicUsize,

    /// If any request has failed.
    ///
    /// Since ssh connection protocol does not return error
    /// for requests, a simple flag is enough.
    request_failed: AtomicBool,

    status: Mutex<Status>,
}

#[derive(Default, Debug)]
enum Status {
    #[default]
    None,

    Waiting(Waker),

    Done,
}

pub(super) enum Completion {
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
    pub(super) async fn start_new_requests(&self, requests: usize) {
        if self.pending_requests.load(Relaxed) != 0 {
            // Wait until all previous requests are processed.
            self.wait_for_completion().await;
        }

        self.pending_requests.store(requests, Relaxed);
        self.request_failed.store(false, Relaxed);

        let prev_status = mem::replace(&mut *self.status.lock().unwrap(), Status::None);

        // Drop prev_status after releasing the lock to reduce critical section.
        drop(prev_status);
    }

    /// Must be called once after `start_new_requests` is called and
    /// data flushed.
    pub(super) fn wait_for_completion(&self) -> impl Future<Output = Completion> + '_ {
        struct WaitForCompletion<'a>(&'a PendingRequests);

        impl Future for WaitForCompletion<'_> {
            type Output = Completion;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut guard = self.0.status.lock().unwrap();

                if let Status::Done = *guard {
                    drop(guard);

                    debug_assert_eq!(self.0.pending_requests.load(Relaxed), 0);

                    return if self.0.request_failed.load(Relaxed) {
                        Poll::Ready(Completion::Failed)
                    } else {
                        Poll::Ready(Completion::Success)
                    };
                }

                let prev_status = mem::replace(&mut *guard, Status::Waiting(cx.waker().clone()));

                // Release the lock
                drop(guard);

                // Drop prev_status after releasing the lock to reduce critical section.
                drop(prev_status);

                Poll::Pending
            }
        }

        WaitForCompletion(self)
    }

    /// Report completion of one request.
    pub(super) fn report_request_completion(&self, completion: Completion) {
        if let Completion::Failed = completion {
            self.request_failed.store(true, Relaxed);
        }

        match self.pending_requests.fetch_sub(1, Relaxed) {
            // Previous value is 1, now it is 0, so perform wakeup
            1 => {
                let prev_status = mem::replace(&mut *self.status.lock().unwrap(), Status::Done);

                debug_assert!(matches!(&prev_status, Status::None | Status::Waiting(..)));

                if let Status::Waiting(waker) = prev_status {
                    waker.wake();
                }
            }
            0 => panic!("Bug: pending_requests OVERFLOWED!"),
            _ => (),
        }
    }
}
