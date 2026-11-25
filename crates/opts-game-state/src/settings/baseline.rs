//! The baseline of the settings: the persisted key/value pairs as they were on disk, which the
//! saves compare with.

use std::collections::BTreeMap;

/// Snapshot of the persisted key/value pairs (the baseline of `GameSettings`, used to save only changed keys).
#[derive(Debug, Default)]
pub struct Baseline(pub std::sync::Arc<std::sync::Mutex<Option<BTreeMap<String, String>>>>);

/// Clones share the baseline: it describes the file, and a copy that is saved on a worker thread
/// (`GameSettings::save_in_background` in `opts-game`) must update what the live settings compare against.
impl Clone for Baseline {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for Baseline {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Baseline {}
