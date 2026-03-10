//! ListReconciler — reconcile Vec<T> into a LoroList using LCS diffing.

use loro::ValueOrContainer;
use similar::algorithms::DiffHook;

use crate::error::ReconcileError;
use crate::hydrate::Hydrate;
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

// ── Vec<T> reconcile — LCS-based ─────────────────────────────────────────

/// Reconcile a `&[T]` into a LoroList using Myers LCS diffing.
///
/// Hydrates old elements for comparison, then emits minimal insert/delete
/// operations. Unchanged regions produce zero CRDT operations.
///
/// Requires `T: Hydrate + PartialEq` for element comparison across
/// representations (Loro → Rust).
pub fn reconcile_vec<T, R>(items: &[T], r: R) -> Result<(), ReconcileError>
where
    T: Reconcile + Hydrate + PartialEq,
    R: Reconciler,
{
    let mut list_r = r.list()?;
    let old_len = list_r.len();

    if old_len == 0 && items.is_empty() {
        return Ok(());
    }

    if old_len == 0 {
        for (i, item) in items.iter().enumerate() {
            list_r.insert(i, item)?;
        }
        return Ok(());
    }

    // Hydrate old elements for comparison.
    // Failed hydrations become Hydrated(None), which never match anything.
    let old: Vec<HydratedItem<T>> = (0..old_len)
        .map(|i| HydratedItem(list_r.get(i).and_then(|voc| T::hydrate(&voc).ok())))
        .collect();

    let mut hook = LcsHook {
        idx: 0,
        list: &mut list_r,
        items,
    };

    // Args swapped: pass new_items as "old", hydrated_old as "new".
    // This satisfies HydratedItem<T>: PartialEq<T> (New::Output: PartialEq<Old::Output>).
    // Hook callbacks are swapped accordingly (delete=insert, insert=delete).
    similar::algorithms::myers::diff(
        &mut hook,
        items, 0..items.len(),
        &old, 0..old.len(),
    )?;

    Ok(())
}

/// Wrapper for hydrated old items. `None` (failed hydration) never equals anything.
struct HydratedItem<T>(Option<T>);

impl<T: PartialEq> PartialEq<T> for HydratedItem<T> {
    fn eq(&self, other: &T) -> bool {
        match &self.0 {
            Some(v) => v == other,
            None => false,
        }
    }
}

/// DiffHook that translates LCS diff operations into LoroList mutations.
///
/// Because `myers::diff` requires `New::Output: PartialEq<Old::Output>` and we
/// can only impl `HydratedItem<T>: PartialEq<T>` (not the reverse, due to orphan
/// rules), we pass the args swapped: `diff(hook, new_items, hydrated_old)`.
///
/// This means the hook's `delete` callback corresponds to items present in
/// `new_items` but absent from `hydrated_old` (= items to INSERT into Loro),
/// and `insert` corresponds to items present in `hydrated_old` but absent from
/// `new_items` (= items to DELETE from Loro).
struct LcsHook<'a, T> {
    /// Current position in the (mutating) LoroList.
    idx: usize,
    list: &'a mut ListReconciler,
    items: &'a [T],
}

impl<T: Reconcile> DiffHook for LcsHook<'_, T> {
    type Error = ReconcileError;

    fn equal(&mut self, _old_index: usize, _new_index: usize, len: usize) -> Result<(), Self::Error> {
        // Items unchanged — skip. No set() on LoroList, so just advance.
        self.idx += len;
        Ok(())
    }

    fn delete(&mut self, old_index: usize, old_len: usize, _new_index: usize) -> Result<(), Self::Error> {
        // "old" in swapped args = new_items. Items in new but not old = INSERT.
        for i in 0..old_len {
            self.list.insert(self.idx, &self.items[old_index + i])?;
            self.idx += 1;
        }
        Ok(())
    }

    fn insert(&mut self, _old_index: usize, new_index: usize, new_len: usize) -> Result<(), Self::Error> {
        let _ = new_index;
        // "new" in swapped args = hydrated_old. Items in old but not new = DELETE.
        for _ in 0..new_len {
            self.list.delete(self.idx)?;
            // Don't advance — subsequent items shift down.
        }
        Ok(())
    }
}

// ── Simple fallback (clear + rewrite) ─────────────────────────────────────

/// Reconcile a `&[T]` into a LoroList using clear + rewrite.
///
/// For types that don't implement `Hydrate + PartialEq`. No diffing —
/// always replaces the entire list contents.
pub fn reconcile_vec_simple<T: Reconcile, R: Reconciler>(
    items: &[T],
    r: R,
) -> Result<(), ReconcileError> {
    let mut list_r = r.list()?;

    while !list_r.is_empty() {
        list_r.delete(0)?;
    }

    for (i, item) in items.iter().enumerate() {
        list_r.insert(i, item)?;
    }

    Ok(())
}

// ── Vec<T> into LoroMovableList ───────────────────────────────────────────

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
