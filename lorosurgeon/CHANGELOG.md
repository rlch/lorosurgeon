# Changelog


### Features

- initial implementation of lorosurgeon
- keyed MovableList LCS reconciliation with CRDT identity preservation
- stale heads detection + O(1) key matching for movable list
- autosurgeon feature parity — Box/Cow/slice impls, enum keys, ByteArray, helpers
- Myers LCS diffing for LoroList reconciliation
- custom attributes, Vec<T> codegen, CI/CD, and polish

### Miscellaneous

- fix all clippy warnings

### Performance

- scalar no-op detection in PropReconciler::put_value
