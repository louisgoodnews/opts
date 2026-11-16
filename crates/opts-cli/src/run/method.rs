//! Method names and the per-method defaults of the command-line client: which calls are slow,
//! how a method written with spaces maps to its dotted name, and the CLI-only defaults.

use crate::commands::find;

/// Methods that legitimately take long (they reply after N frames or after loading).
pub(super) fn is_slow(method: &str) -> bool {
    matches!(
        method,
        "sim.step"
            | "scene.load_map"
            | "scene.unload"
            | "vehicle.set"
            | "ui.wait_for"
            | "ui.drag"
            | "ui.text"
            | "ui.select"
            | "camera.look"
            | "snapshot"
            | "screenshot"
            | "capture.burst"
            | "capture.record.stop"
            | "report"
            | "batch"
    )
}

/// Splits the command words into the method and its arguments. A method may be written with
/// spaces instead of dots (`capture record start` for `capture.record.start`): the longest
/// leading run of words whose dotted join is a known command wins.
pub(super) fn split_method(words: &[String]) -> Option<(String, &[String])> {
    let (first, rest) = words.split_first()?;
    if find(first).is_none() {
        for n in (2..=words.len().min(3)).rev() {
            let joined = words[..n].join(".");
            if find(&joined).is_some() {
                return Some((joined, &words[n..]));
            }
        }
    }
    Some((first.clone(), rest))
}

/// Defaults that only the command-line client applies (the protocol's own defaults differ): a
/// recording started from a terminal is for quick iteration, so it lasts 30 s, not 60 s.
pub(super) fn apply_cli_defaults(method: &str, params: &mut serde_json::Value) {
    if method == "capture.record.start" {
        if let Some(obj) = params.as_object_mut() {
            obj.entry("duration_ms")
                .or_insert(serde_json::json!(30_000));
        } else if params.is_null() {
            *params = serde_json::json!({"duration_ms": 30_000});
        }
    }
}
