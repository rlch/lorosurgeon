//! Counter wrapper — CRDT counter backed by LoroCounter.
//!
//! Only available when the `counter` feature is enabled.

#[cfg(feature = "counter")]
mod inner {
    use loro::counter::LoroCounter;

    use crate::error::{HydrateError, ReconcileError};
    use crate::hydrate::Hydrate;
    use crate::reconcile::{NoKey, Reconcile, Reconciler};

    /// A CRDT counter value. Tracks current value and original (rehydrated) value
    /// so that reconciliation can produce increments rather than absolute sets.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Counter {
        pub current: f64,
        /// `None` for fresh counters, `Some(original)` for rehydrated.
        original: Option<f64>,
    }

    impl Counter {
        /// Create a new counter with the given value.
        pub fn new(value: f64) -> Self {
            Self {
                current: value,
                original: None,
            }
        }

        /// Create a counter rehydrated from a Loro document.
        pub fn rehydrated(value: f64) -> Self {
            Self {
                current: value,
                original: Some(value),
            }
        }

        /// Increment the counter by the given amount.
        pub fn increment(&mut self, by: f64) {
            self.current += by;
        }

        /// Get the current value.
        pub fn value(&self) -> f64 {
            self.current
        }
    }

    impl Default for Counter {
        fn default() -> Self {
            Self::new(0.0)
        }
    }

    impl Hydrate for Counter {
        fn hydrate_counter(counter: &LoroCounter) -> Result<Self, HydrateError> {
            Ok(Counter::rehydrated(counter.get()))
        }

        fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
            Ok(Counter::rehydrated(f))
        }

        fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
            Ok(Counter::rehydrated(i as f64))
        }
    }

    impl Reconcile for Counter {
        type Key<'a> = NoKey;

        fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
            let mut c = r.counter().map_err(Into::into)?;
            match self.original {
                Some(orig) => c.increment(self.current - orig)?,
                None => c.set(self.current)?,
            }
            Ok(())
        }
    }

    /// Sub-reconciler for LoroCounter.
    pub struct CounterReconciler {
        pub(crate) counter: LoroCounter,
    }

    impl CounterReconciler {
        /// Set the counter to an absolute value.
        pub fn set(&mut self, value: f64) -> Result<(), ReconcileError> {
            let current = self.counter.get();
            self.counter.increment(value - current)?;
            Ok(())
        }

        /// Increment the counter by the given amount.
        pub fn increment(&mut self, by: f64) -> Result<(), ReconcileError> {
            self.counter.increment(by)?;
            Ok(())
        }

        /// Get the current value.
        pub fn get(&self) -> f64 {
            self.counter.get()
        }
    }
}

#[cfg(feature = "counter")]
pub use inner::*;
