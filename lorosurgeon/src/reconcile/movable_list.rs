//! MovableListReconciler — reconcile Vec<T> into a LoroMovableList.

use loro::ValueOrContainer;

use crate::error::ReconcileError;
use crate::reconcile::{MovableListReconciler, PropReconciler, Reconcile};

impl MovableListReconciler {
    /// Get an item at the given index.
    pub fn get(&self, index: usize) -> Option<ValueOrContainer> {
        self.list.get(index)
    }

    /// Set a value at the given index (in-place update).
    pub fn set<R: Reconcile>(&mut self, index: usize, value: &R) -> Result<(), ReconcileError> {
        let reconciler = PropReconciler::movable_list_set(self.list.clone(), index);
        value.reconcile(reconciler)
    }

    /// Insert a value at the given index.
    pub fn insert<R: Reconcile>(
        &mut self,
        index: usize,
        value: &R,
    ) -> Result<(), ReconcileError> {
        let reconciler = PropReconciler::movable_list_insert(self.list.clone(), index);
        value.reconcile(reconciler)
    }

    /// Delete an item at the given index.
    pub fn delete(&mut self, index: usize) -> Result<(), ReconcileError> {
        self.list.delete(index, 1)?;
        Ok(())
    }

    /// Move an item from one position to another.
    pub fn mov(&mut self, from: usize, to: usize) -> Result<(), ReconcileError> {
        self.list.mov(from, to)?;
        Ok(())
    }

    /// Get the length of the list.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.list.len() == 0
    }
}
