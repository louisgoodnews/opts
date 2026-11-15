//! Validation of a batch of option values: every refusal is listed, nothing is applied.

use opts_cli::protocol::code;
use opts_core::setvars::{SetvarCatalogue, SetvarEntry, SetvarKind};
use serde_json::{json, Map, Value};

/// What a value must be for `e`, in words.
pub(super) fn expectation(e: &SetvarEntry) -> String {
    match e.kind {
        SetvarKind::Switch => "0 or 1".into(),
        SetvarKind::Choice => format!("one of {:?}", e.values),
        SetvarKind::Number => format!("a whole number from {} to {}", e.min, e.max),
        SetvarKind::Fraction => "not settable here: only the repaint sets decimal options".into(),
    }
}

/// Validates the batch against the catalogue; every refusal is listed, nothing is applied.
pub(super) fn validate(
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Result<Vec<(String, i32)>, Vec<Value>> {
    let (mut ok, mut bad) = (Vec::new(), Vec::new());
    for (name, v) in values {
        let refuse =
            |why: String| json!({ "key": name, "code": code::INVALID_PARAMS, "message": why });
        let number = match v {
            Value::Number(n) => n.as_f64(),
            Value::Bool(b) => Some(f64::from(u8::from(*b))),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        let Some(number) = number.filter(|n| n.is_finite() && n.fract() == 0.0) else {
            bad.push(refuse(format!("{name}: expected a whole number")));
            continue;
        };
        match catalogue.get(name) {
            Some(e) if e.accepts(number) => ok.push((e.name.clone(), number as i32)),
            Some(e) => bad.push(refuse(format!("{}: expected {}", e.name, expectation(e)))),
            None if allow_unlisted => ok.push((name.trim().to_string(), number as i32)),
            None => bad.push(refuse(format!(
                "{name}: not an option of this vehicle (allow_unlisted: true sets it anyway; the vendors warn that unlisted variables can cause faults)"
            ))),
        }
    }
    if bad.is_empty() {
        Ok(ok)
    } else {
        Err(bad)
    }
}
