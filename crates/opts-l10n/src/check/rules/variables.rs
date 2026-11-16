//! Variable checks of the validation rules.

use super::*;

pub(super) fn validate_variables(
    pattern: &Pattern,
    key: &str,
    reference: &Reference,
) -> Result<(), (&'static str, String)> {
    let have = fluent::variables(pattern);
    let want = reference.variables_of(key).cloned().unwrap_or_default();
    let is_var = |s: &&String| s.starts_with('$');
    let lost: Vec<&String> = want
        .iter()
        .filter(is_var)
        .filter(|v| !have.contains(*v))
        .collect();
    let added: Vec<&String> = have
        .iter()
        .filter(is_var)
        .filter(|v| !want.contains(*v))
        .collect();
    if !lost.is_empty() || !added.is_empty() {
        let mut why = String::new();
        if !lost.is_empty() {
            why.push_str(&format!(
                "missing {}; ",
                lost.iter()
                    .map(|s| format!("`{{ {s} }}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !added.is_empty() {
            why.push_str(&format!(
                "unknown {}; ",
                added
                    .iter()
                    .map(|s| format!("`{{ {s} }}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        return Err((
            "variable-mismatch",
            format!(
                "the variables differ from English ({why}English uses {}). Keep every `{{ $name }}` exactly as written",
                if want.is_empty() {
                    "none".to_string()
                } else {
                    want.iter().filter(is_var).cloned().collect::<Vec<_>>().join(", ")
                }
            ),
        ));
    }
    let unknown_refs: Vec<&String> = have
        .iter()
        .filter(|s| !s.starts_with('$') && !reference.table.messages.contains_key(*s))
        .collect();
    if !unknown_refs.is_empty() {
        return Err((
            "unknown-reference",
            format!("refers to message(s) that do not exist: {unknown_refs:?}"),
        ));
    }
    Ok(())
}
