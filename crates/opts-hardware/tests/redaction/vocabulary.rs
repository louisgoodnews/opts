//! The closed vocabulary a serialised profile may use, and the check that enforces it.

use opts_hardware::probe::{GPU_FEATURES, ISA_FEATURES};
use opts_hardware::Reason;
use serde_json::Value;

use super::patterns::is_hex;

pub(super) fn allowed_words() -> Vec<String> {
    let mut v: Vec<String> = Reason::ALL.iter().map(|r| r.key().to_owned()).collect();
    v.extend(
        ISA_FEATURES
            .iter()
            .chain(GPU_FEATURES)
            .map(|s| (*s).to_owned()),
    );
    // Enum keys: take them from the generated vocabulary of the reference page.
    v.extend(
        opts_hardware::reference::render()
            .lines()
            .filter(|l| l.starts_with("* `"))
            .flat_map(|l| {
                l.split('`')
                    .skip(1)
                    .step_by(2)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            }),
    );
    v
}

/// Every string in the document is a field name, an enum key, a feature name, a reason or the
/// 16-digit class hash; everything else is a number, a bool or null.
pub(super) fn assert_closed_vocabulary(v: &Value, allowed: &[String]) {
    match v {
        Value::String(s) => assert!(
            allowed.contains(s) || (s.len() == 16 && is_hex(s)),
            "string {s:?} is outside the vocabulary"
        ),
        Value::Array(a) => a.iter().for_each(|x| assert_closed_vocabulary(x, allowed)),
        Value::Object(m) => m
            .values()
            .for_each(|x| assert_closed_vocabulary(x, allowed)),
        _ => {}
    }
}
