use std::{
    future::Future,
    mem,
    num::NonZeroU64,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering::Relaxed},
        Mutex,
    },
    task::{Context, Poll, Waker},
};

/// AwaitableAtomicU64.
/// Can have multiple writer that adds to the counter but only one reader
/// that decrements the counter.
#[derive(Debug, Default)]
pub(crate) struct AwaitableAtomicU64 {
    atomic: AtomicU64,

    waker: Mutex<Option<Waker>>,
}

/// For the reader
impl AwaitableAtomicU64 {
    /// Return the previous value and set the atomic to 0.
    pub(crate) fn get(&self) -> u64 {
        self.atomic.swap(0, Relaxed)
    }

    fn get_non_zero(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.get())
    }

    /// Return the atomic value if it is non-zero,
    /// or wait until it is changed to non-zero.
    ///
    /// It will set the atomic to 0 atomically before returning.
    pub(crate) fn wait_until_non_zero(&self) -> impl Future<Output = NonZeroU64> + '_ {
        struct WaitUntilNonZero<'a>(&'a AwaitableAtomicU64);

        impl Future for WaitUntilNonZero<'_> {
            type Output = NonZeroU64;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.0;

                // Point 1
                if let Some(int) = this.get_non_zero() {
                    return Poll::Ready(int);
                }

                // Point 2
                let mut guard = this.waker.lock().unwrap();

                // Retest the condition since [`AtomicU64::add`] might be called
                // between point 1 and point 2.
                if let Some(int) = this.get_non_zero() {
                    return Poll::Ready(int);
                }

                // Now that we have tested that [`AtomicU64::add`] is not called
                // just before point 2, we can register the waker here.
                //
                // Any [`AtomicU64::add`] called after point 2 will wake us up.
                let prev_waker = mem::replace(&mut *guard, Some(cx.waker().clone()));

                // Release lock
                drop(guard);

                drop(prev_waker);

                // One final test to avoid yielding if possible.
                if let Some(int) = this.get_non_zero() {
                    Poll::Ready(int)
                } else {
                    Poll::Pending
                }
            }
        }

        WaitUntilNonZero(self)
    }
}

/// For the writers
impl AwaitableAtomicU64 {
    pub(crate) fn add(&self, val: u64) {
        let prev_value = self.atomic.fetch_add(val, Relaxed);
        let new_value = prev_value + val;

        if new_value < prev_value {
            // Technically this panic is unnecessary, since calcaultion of
            // new_value alrdady checks for overflowing.
            panic!("u64 is overflowed!")
        }

        let waker = self.waker.lock().unwrap().take();

        // Cal waker here to reduce critical section
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
