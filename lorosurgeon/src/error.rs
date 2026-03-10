//! Error types for hydration and reconciliation.

#[derive(Debug, thiserror::Error)]
pub enum HydrateError {
    #[error(transparent)]
    Loro(#[from] loro::LoroError),

    #[error("expected {expected}, found {found}")]
    Unexpected {
        expected: &'static str,
        found: &'static str,
    },

    #[error("missing required property: {key}")]
    Missing { key: String },

    #[error("json deserialization failed for {key}: {source}")]
    Json {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("integer overflow: {value} doesn't fit in {target_type}")]
    Overflow {
        value: i64,
        target_type: &'static str,
    },
}

impl HydrateError {
    pub fn unexpected(expected: &'static str, found: &'static str) -> Self {
        Self::Unexpected { expected, found }
    }

    pub fn missing(key: impl Into<String>) -> Self {
        Self::Missing { key: key.into() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReconcileError {
    #[error(transparent)]
    Loro(#[from] loro::LoroError),

    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("type mismatch: expected {expected} container, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    #[error("stale heads: document was modified during reconciliation")]
    StaleHeads,
}
