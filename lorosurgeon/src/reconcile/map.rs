//! MapReconciler — reconcile struct fields and HashMap entries into a LoroMap.

use loro::ValueOrContainer;

use crate::error::ReconcileError;
use crate::reconcile::{MapReconciler, PropReconciler, Reconcile};

impl MapReconciler {
    /// Write a field/entry value into the map at the given key.
    pub fn entry<R: Reconcile>(&mut self, key: &str, value: &R) -> Result<(), ReconcileError> {
        let reconciler = PropReconciler::map_put(self.map.clone(), key.to_string());
        value.reconcile(reconciler)
    }

    /// Delete a key from the map.
    pub fn delete(&mut self, key: &str) -> Result<(), ReconcileError> {
        self.map.delete(key)?;
        Ok(())
    }

    /// Retain only keys matching the predicate, deleting the rest.
    pub fn retain(
        &mut self,
        mut pred: impl FnMut(&str) -> bool,
    ) -> Result<(), ReconcileError> {
        let keys_to_delete: Vec<String> = self
            .keys()
            .filter(|k| !pred(k))
            .collect();
        for key in keys_to_delete {
            self.map.delete(&key)?;
        }
        Ok(())
    }

    /// Iterate over keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = String> {
        let mut keys = Vec::new();
        self.map.for_each(|key, _| {
            keys.push(key.to_string());
        });
        keys.into_iter()
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<ValueOrContainer> {
        self.map.get(key)
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }
}

// ── HashMap / BTreeMap Reconcile ────────────────────────────────────────

use std::collections::{BTreeMap, HashMap};

use crate::reconcile::{NoKey, Reconciler};

impl<V: Reconcile> Reconcile for HashMap<String, V> {
    type Key = NoKey;

    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        let mut m = r.map()?;
        for (key, value) in self {
            m.entry(key, value)?;
        }
        let new_keys: std::collections::HashSet<&str> =
            self.keys().map(|k| k.as_str()).collect();
        m.retain(|k| new_keys.contains(k))?;
        Ok(())
    }
}

impl<V: Reconcile> Reconcile for BTreeMap<String, V> {
    type Key = NoKey;

    fn reconcile<R: Reconciler>(&self, r: R) -> Result<(), ReconcileError> {
        let mut m = r.map()?;
        for (key, value) in self {
            m.entry(key, value)?;
        }
        let new_keys: std::collections::HashSet<&str> =
            self.keys().map(|k| k.as_str()).collect();
        m.retain(|k| new_keys.contains(k))?;
        Ok(())
    }
}
