//! MovableListReconciler — reconcile Vec<T> into a LoroMovableList with LCS diffing.

use loro::ValueOrContainer;

use crate::error::ReconcileError;
use crate::reconcile::{LoadKey, MovableListReconciler, PropReconciler, Reconcile};

impl MovableListReconciler {
    /// Get an item at the given index.
    pub fn get(&self, index: usize) -> Option<ValueOrContainer> {
        self.list.get(index)
    }

    /// Set a value at the given index (in-place update, preserves CRDT identity).
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

/// Reconcile a `Vec<T>` into a `LoroMovableList` using key-based diffing.
///
/// - Items with `#[key]`: matched by key identity. Matched items use `set()`
///   in place (preserving CRDT element identity). Unmatched items are
///   `insert`/`delete`d.
/// - Items without keys: positional set/truncate/append.
pub fn reconcile_movable_list<T: Reconcile>(
    items: &[T],
    list_r: &mut MovableListReconciler,
) -> Result<(), ReconcileError> {
    let old_len = list_r.len();

    // Check if items have keys
    let has_keys = items.first().map_or(false, |item| {
        !matches!(item.key(), LoadKey::NoKey)
    });

    if !has_keys {
        return reconcile_positional(items, list_r, old_len);
    }

    reconcile_keyed(items, list_r, old_len)
}

/// No keys — positional set/truncate/append.
fn reconcile_positional<T: Reconcile>(
    items: &[T],
    list_r: &mut MovableListReconciler,
    old_len: usize,
) -> Result<(), ReconcileError> {
    let overlap = old_len.min(items.len());
    for i in 0..overlap {
        list_r.set(i, &items[i])?;
    }

    // Remove extras from the end
    for _ in items.len()..old_len {
        list_r.delete(items.len())?;
    }

    // Append new items
    for i in old_len..items.len() {
        list_r.insert(i, &items[i])?;
    }

    Ok(())
}

/// Key-based reconciliation.
///
/// Strategy:
/// 1. Hydrate keys from old (Loro) side, extract keys from new (Rust) side.
/// 2. Match new items to old items by key identity.
/// 3. Delete unmatched old items (back-to-front to preserve indices).
/// 4. Build target order using a tracking vec. For each new item:
///    - If matched: find its current position and mov() if needed, then set().
///    - If new: insert() at target position.
///
/// We maintain a `current_order` vec that mirrors the Loro list's state after
/// each operation, so we always know exact positions.
fn reconcile_keyed<T: Reconcile>(
    items: &[T],
    list_r: &mut MovableListReconciler,
    old_len: usize,
) -> Result<(), ReconcileError> {
    // Hydrate keys from old (Loro) side.
    let old_keys: Vec<Option<T::Key>> = (0..old_len)
        .map(|i| {
            list_r
                .get(i)
                .and_then(|voc| T::hydrate_key(&voc).ok())
                .and_then(|lk| lk.into_found())
        })
        .collect();

    // Extract keys from new (Rust) side.
    let new_keys: Vec<Option<T::Key>> = items
        .iter()
        .map(|item| item.key().into_found())
        .collect();

    // Match each new item to an old item by key.
    let mut old_used = vec![false; old_len];
    let mut new_to_old: Vec<Option<usize>> = Vec::with_capacity(items.len());

    for new_key in &new_keys {
        let matched = new_key.as_ref().and_then(|nk| {
            (0..old_len).find(|&i| {
                !old_used[i] && old_keys[i].as_ref().map_or(false, |ok| ok == nk)
            })
        });

        if let Some(old_idx) = matched {
            old_used[old_idx] = true;
            new_to_old.push(Some(old_idx));
        } else {
            new_to_old.push(None);
        }
    }

    // Phase 1: Delete unmatched old items (back-to-front).
    let delete_indices: Vec<usize> = (0..old_len)
        .filter(|i| !old_used[*i])
        .collect();
    for &idx in delete_indices.iter().rev() {
        list_r.delete(idx)?;
    }

    // Build a tracking vec: current_order[i] = original old_index of item at position i.
    // After deletions, only surviving items remain in their original relative order.
    let mut current_order: Vec<usize> = (0..old_len)
        .filter(|i| old_used[*i])
        .collect();

    // Phase 2: Walk new items in order, building the target arrangement.
    for (target_idx, maybe_old) in new_to_old.iter().enumerate() {
        match maybe_old {
            Some(old_idx) => {
                // Find where this old item currently sits.
                let current_pos = current_order
                    .iter()
                    .position(|&x| x == *old_idx)
                    .expect("matched item must exist in current_order");

                if current_pos != target_idx {
                    list_r.mov(current_pos, target_idx)?;
                    // Update tracking: remove from current_pos, insert at target_idx.
                    let val = current_order.remove(current_pos);
                    current_order.insert(target_idx, val);
                }
                // Set in place — preserves CRDT element identity.
                list_r.set(target_idx, &items[target_idx])?;
            }
            None => {
                // New item — insert at target position.
                list_r.insert(target_idx, &items[target_idx])?;
                // Use a sentinel value (usize::MAX) since this is a new item with no old_idx.
                current_order.insert(target_idx, usize::MAX);
            }
        }
    }

    Ok(()
    )
}
