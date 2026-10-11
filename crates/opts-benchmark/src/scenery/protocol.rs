//! Line protocol between the supervisor and its workers.
//!
//! The supervisor writes one [`ObjectEntry`](super::ObjectEntry) as JSON per line to a
//! worker's stdin (end of input means "stop"). The worker answers on stdout with lines starting with [`PREFIX`] followed by a
//! JSON [`WorkerMsg`]; any other stdout line is ignored, so stray output of native libraries
//! cannot break the protocol.

use serde::{Deserialize, Serialize};

use super::Record;

/// Prefix of every protocol line on a worker's stdout.
pub const PREFIX: &str = "@@opts-benchmark ";

/// A message from a worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMsg {
    /// The worker has loaded the generated map and waits for objects.
    Ready,
    /// The worker started an object.
    Start {
        /// Object key.
        key: String,
    },
    /// The record of the object the worker just finished.
    Result {
        /// The record.
        record: Box<Record>,
    },
    /// The worker exits on purpose (recycling, or after a soft timeout) and wants a successor.
    Recycle {
        /// Why.
        reason: String,
    },
}

impl WorkerMsg {
    /// The protocol line for this message.
    #[must_use]
    pub fn to_line(&self) -> String {
        format!(
            "{PREFIX}{}",
            serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
        )
    }

    /// Parses a stdout line; `None` for lines that are not protocol lines.
    #[must_use]
    pub fn from_line(line: &str) -> Option<Self> {
        serde_json::from_str(line.strip_prefix(PREFIX)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip_and_noise_is_ignored() {
        let m = WorkerMsg::Start { key: "abc".into() };
        assert_eq!(WorkerMsg::from_line(&m.to_line()), Some(m));
        assert_eq!(WorkerMsg::from_line("vulkan: some driver chatter"), None);
    }
}
