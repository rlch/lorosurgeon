//! ListReconciler — reconcile Vec<T> into a LoroList using LCS diffing.

use loro::ValueOrContainer;

use crate::error::ReconcileError;
use crate::reconcile::{ListReconciler, PropReconciler, Reconcile, Reconciler};

impl ListReconciler {
    /// Get an item at the given index.
    pub fn get(&self, index: usize) -> Option<ValueOrContainer> {
        self.list.get(index)
    }

    /// Insert a value at the given index.
    pub fn insert<R: Reconcile>(&mut self, index: usize, value: &R) -> Result<(), ReconcileError> {
        let reconciler = PropReconciler::list_insert(self.list.clone(), index);
        value.reconcile(reconciler)
    }

    /// Delete an item at the given index.
    pub fn delete(&mut self, index: usize) -> Result<(), ReconcileError> {
        self.list.delete(index, 1)?;
        Ok(())
    }

    /// Get the length of the list.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

// ── Vec<T> reconcile helper ─────────────────────────────────────────────

/// Reconcile a Vec<T> into a LoroList.
/// Uses simple positional strategy (clear + rewrite) since LoroList
/// has no `set` method for in-place updates.
pub fn reconcile_vec<T: Reconcile, R: Reconciler>(
    items: &[T],
    r: R,
) -> Result<(), ReconcileError> {
    let mut list_r = r.list()?;

    // Clear existing items
    while !list_r.is_empty() {
        list_r.delete(0)?;
    }

    // Insert all new items
    for (i, item) in items.iter().enumerate() {
        list_r.insert(i, item)?;
    }

    Ok(())
}

/// Reconcile a Vec<T> into a LoroMovableList using LCS-based diffing.
///
/// Items with `#[key]` are matched by key identity — matched items are
/// updated in place via `set()`, preserving CRDT element identity.
/// Items without keys use positional set/insert/delete.
pub fn reconcile_vec_movable<T: Reconcile, R: Reconciler>(
    items: &[T],
    r: R,
) -> Result<(), ReconcileError> {
    let mut list_r = r.movable_list()?;
    super::movable_list::reconcile_movable_list(items, &mut list_r)
}

/// Wrapper to reconcile a `&[T]` into a `LoroMovableList` via `MapReconciler::entry()`.
pub struct MovableVec<'a, T>(pub &'a [T]);

impl<T: Reconcile> Reconcile for MovableVec<'_, T> {
    type Key = crate::reconcile::NoKey;

    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        reconcile_vec_movable(self.0, r)
    }
}
