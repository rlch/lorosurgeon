//! Text wrapper — plain text backed by LoroText with built-in LCS diffing.

use loro::{LoroText, UpdateOptions};

use crate::error::{HydrateError, ReconcileError};
use crate::hydrate::Hydrate;
use crate::reconcile::{NoKey, Reconcile, Reconciler};

/// Plain text stored in a LoroText container.
///
/// Uses Loro's built-in `update()` method which performs LCS diffing
/// to produce minimal insert/delete operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Text(pub String);

impl Text {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Hydrate for Text {
    fn hydrate_text(text: &LoroText) -> Result<Self, HydrateError> {
        Ok(Text(text.to_string()))
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        Ok(Text(s.to_string()))
    }
}

impl Reconcile for Text {
    type Key<'a> = NoKey;

    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        let mut t = r.text()?;
        t.update(&self.0)?;
        Ok(())
    }
}

impl crate::reconcile::TextReconciler {
    /// Update the text content using Loro's built-in LCS diff.
    pub fn update(&mut self, new_text: &str) -> Result<(), ReconcileError> {
        self.text
            .update(new_text, UpdateOptions::default())
            .map_err(|_e| ReconcileError::TypeMismatch {
                expected: "text update",
                found: "timeout",
            })?;
        Ok(())
    }

    /// Get current text content.
    pub fn get(&self) -> String {
        self.text.to_string()
    }
}
