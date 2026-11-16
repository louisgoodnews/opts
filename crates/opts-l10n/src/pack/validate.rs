//! Field validation of the pack metadata file.

use super::*;

pub(super) fn fail_field(issues: &mut Vec<Issue>, field: &str, line: Option<usize>, why: &str) {
    issues.push(Issue::new(
        Severity::Error,
        "pack-json",
        FILE,
        line,
        None,
        format!("`{field}`: {why}"),
    ));
}

pub(super) fn extract_text_field(
    obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    required: bool,
    issues: &mut Vec<Issue>,
    line: Option<usize>,
) -> String {
    match obj.get(field) {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
        Some(serde_json::Value::String(_)) | None if required => {
            fail_field(issues, field, line, "required, a non-empty text");
            String::new()
        }
        Some(serde_json::Value::String(s)) => s.clone(),
        None => String::new(),
        Some(_) => {
            fail_field(issues, field, line, "must be text");
            String::new()
        }
    }
}

pub(super) fn parse_authors(
    obj: &serde_json::Map<String, serde_json::Value>,
    issues: &mut Vec<Issue>,
    line: Option<usize>,
) -> Vec<String> {
    match obj.get("authors") {
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
        Some(_) => {
            fail_field(
                issues,
                "authors",
                line,
                "must be a list of names, for example [\"Jane Doe\"]",
            );
            Vec::new()
        }
    }
}

pub(super) fn validate_code(code: &str, folder: &str, issues: &mut Vec<Issue>, line: Option<usize>) {
    if code.is_empty() {
        return;
    }
    if !valid_code(code) {
        fail_field(
            issues,
            "code",
            line,
            "must look like `fr` or `pt-BR` (lower-case language, optional region)",
        );
    } else if code != folder {
        fail_field(
            issues,
            "code",
            line,
            &format!("is `{code}` but the folder is `{folder}`; they must be equal"),
        );
    } else if code == "en-XA" || code.eq_ignore_ascii_case("en-xa") {
        fail_field(
            issues,
            "code",
            line,
            "`en-XA` is the pseudo-locale of the tests and cannot be a pack",
        );
    }
}
