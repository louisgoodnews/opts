//! The host handler type and the per-callback statistics of the registry.

use std::collections::BTreeMap;

use super::{CallbackArgs, CallbackResult};

/// The host's implementation of the callbacks: receives the lower-case name and the popped
/// arguments; returns `None` for a callback it does not implement (the registry then answers
/// with the stub and reports it once).
pub type CallbackHandler = Box<dyn FnMut(&str, &CallbackArgs) -> Option<CallbackResult> + Send>;

/// How often each callback was answered, by name: by the host, by the stub, or unknown to the
/// registry (no signature: nothing popped or pushed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallbackStats {
    /// Calls the host implemented.
    pub real: BTreeMap<String, u64>,
    /// Calls the stub answered.
    pub stub: BTreeMap<String, u64>,
    /// Calls to names outside OMSI's lists.
    pub unknown: BTreeMap<String, u64>,
}

impl CallbackStats {
    /// Names of the callbacks the stub answered.
    #[must_use]
    pub fn stubbed(&self) -> Vec<String> {
        self.stub.keys().cloned().collect()
    }
}
