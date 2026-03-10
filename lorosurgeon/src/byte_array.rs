//! Fixed-size byte array wrapper for Loro binary round-trips.

use crate::error::{HydrateError, ReconcileError};
use crate::hydrate::Hydrate;
use crate::reconcile::{NoKey, Reconcile, Reconciler};

/// A fixed-size byte array that round-trips through Loro's binary type.
///
/// ```ignore
/// #[derive(Hydrate, Reconcile)]
/// struct Record {
///     checksum: ByteArray<32>,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ByteArray<const N: usize>(pub [u8; N]);

impl<const N: usize> ByteArray<N> {
    pub fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> From<[u8; N]> for ByteArray<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

impl<const N: usize> Hydrate for ByteArray<N> {
    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        let arr: [u8; N] = b.try_into().map_err(|_| HydrateError::Unexpected {
            expected: "binary of correct length",
            found: "binary of wrong length",
        })?;
        Ok(Self(arr))
    }
}

impl<const N: usize> Reconcile for ByteArray<N> {
    type Key = NoKey;
    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        r.bytes(&self.0)
    }
}

// ── uuid feature ──────────────────────────────────────────────────────

#[cfg(feature = "uuid")]
mod uuid_impl {
    use super::*;

    impl Hydrate for uuid::Uuid {
        fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
            uuid::Uuid::from_slice(b).map_err(|_| HydrateError::Unexpected {
                expected: "16-byte binary (UUID)",
                found: "binary of wrong length",
            })
        }

        fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
            uuid::Uuid::parse_str(s).map_err(|_| HydrateError::Unexpected {
                expected: "valid UUID string",
                found: "invalid UUID string",
            })
        }
    }

    impl Reconcile for uuid::Uuid {
        type Key = NoKey;
        fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
            r.bytes(self.as_bytes())
        }
    }
}
