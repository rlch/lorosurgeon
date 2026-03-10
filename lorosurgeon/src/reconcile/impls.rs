//! Reconcile implementations for built-in types.

use crate::error::ReconcileError;
use crate::reconcile::{LoadKey, NoKey, Reconcile, Reconciler};

// ── Boolean ─────────────────────────────────────────────────────────────

impl Reconcile for bool {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.boolean(*self)
    }
}

// ── Signed integers ─────────────────────────────────────────────────────

macro_rules! impl_reconcile_int {
    ($($t:ty),*) => {
        $(
            impl Reconcile for $t {
                type Key = NoKey;
                fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
                    r.i64(*self as i64)
                }
            }
        )*
    };
}

impl_reconcile_int!(i8, i16, i32, i64, u8, u16, u32, u64, usize);

// ── Floating point ──────────────────────────────────────────────────────

impl Reconcile for f64 {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.f64(*self)
    }
}

impl Reconcile for f32 {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.f64(*self as f64)
    }
}

// ── String ──────────────────────────────────────────────────────────────

impl Reconcile for String {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.str(self)
    }
}

// ── Vec<u8> (Binary) ────────────────────────────────────────────────────

impl Reconcile for Vec<u8> {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.bytes(self)
    }
}

// ── Option<T> ───────────────────────────────────────────────────────────

impl<T: Reconcile> Reconcile for Option<T> {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        match self {
            None => r.null(),
            Some(v) => v.reconcile(r),
        }
    }
}

// ── serde_json::Value ───────────────────────────────────────────────────

impl Reconcile for serde_json::Value {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        let s = serde_json::to_string(self)?;
        r.str(&s)
    }
}

// ── &str ──────────────────────────────────────────────────────────────

impl Reconcile for &str {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.str(self)
    }
}

// ── &[T] ──────────────────────────────────────────────────────────────

impl<T: Reconcile> Reconcile for &[T] {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        super::list::reconcile_vec(self, r)
    }
}

// ── Box<T> ────────────────────────────────────────────────────────────

impl<T: Reconcile> Reconcile for Box<T> {
    type Key = T::Key;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        (**self).reconcile(r)
    }

    fn key(&self) -> LoadKey<Self::Key> {
        (**self).key()
    }
}

// ── Cow<'a, T> ────────────────────────────────────────────────────────

impl<'a, T: Reconcile + Clone + 'a> Reconcile for std::borrow::Cow<'a, T> {
    type Key = T::Key;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        self.as_ref().reconcile(r)
    }

    fn key(&self) -> LoadKey<Self::Key> {
        self.as_ref().key()
    }
}

// ── Reconcile for &T ────────────────────────────────────────────────────

impl<'b, T: Reconcile + 'b> Reconcile for &'b T {
    type Key = T::Key;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        (*self).reconcile(r)
    }

    fn key(&self) -> LoadKey<Self::Key> {
        (*self).key()
    }
}
