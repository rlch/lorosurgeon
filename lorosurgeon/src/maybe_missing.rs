//! MaybeMissing<T> — distinguishes "key absent" from "key present".

use loro::{LoroList, LoroMap, LoroMovableList, ValueOrContainer};

use crate::error::{HydrateError, ReconcileError};
use crate::hydrate::Hydrate;
use crate::reconcile::{NoKey, Reconcile, Reconciler};

/// Tracks whether a value was absent or present in the Loro document.
///
/// Unlike `Option<T>`, which maps both "missing key" and "null value" to `None`,
/// `MaybeMissing<T>` only returns `Missing` when the key is truly absent.
/// A null value would attempt to hydrate `T` and may error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeMissing<T> {
    Missing,
    Present(T),
}

impl<T> MaybeMissing<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, MaybeMissing::Missing)
    }

    pub fn is_present(&self) -> bool {
        matches!(self, MaybeMissing::Present(_))
    }

    pub fn as_ref(&self) -> MaybeMissing<&T> {
        match self {
            MaybeMissing::Missing => MaybeMissing::Missing,
            MaybeMissing::Present(v) => MaybeMissing::Present(v),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            MaybeMissing::Missing => default,
            MaybeMissing::Present(v) => v,
        }
    }

    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            MaybeMissing::Missing => T::default(),
            MaybeMissing::Present(v) => v,
        }
    }
}

impl<T: Default> Default for MaybeMissing<T> {
    fn default() -> Self {
        MaybeMissing::Missing
    }
}

impl<T: Hydrate> Hydrate for MaybeMissing<T> {
    fn hydrate(source: &ValueOrContainer) -> Result<Self, HydrateError> {
        T::hydrate(source).map(MaybeMissing::Present)
    }

    fn hydrate_null() -> Result<Self, HydrateError> {
        Ok(MaybeMissing::Missing)
    }

    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        T::hydrate_bool(b).map(MaybeMissing::Present)
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        T::hydrate_i64(i).map(MaybeMissing::Present)
    }

    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        T::hydrate_f64(f).map(MaybeMissing::Present)
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        T::hydrate_string(s).map(MaybeMissing::Present)
    }

    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        T::hydrate_binary(b).map(MaybeMissing::Present)
    }

    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        T::hydrate_map(map).map(MaybeMissing::Present)
    }

    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        T::hydrate_list(list).map(MaybeMissing::Present)
    }

    fn hydrate_movable_list(list: &LoroMovableList) -> Result<Self, HydrateError> {
        T::hydrate_movable_list(list).map(MaybeMissing::Present)
    }

    fn hydrate_text(text: &loro::LoroText) -> Result<Self, HydrateError> {
        T::hydrate_text(text).map(MaybeMissing::Present)
    }
}

impl<T: Reconcile> Reconcile for MaybeMissing<T> {
    type Key<'a> = NoKey where T: 'a;

    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        match self {
            MaybeMissing::Missing => r.null().map_err(Into::into),
            MaybeMissing::Present(v) => v.reconcile(r),
        }
    }
}
