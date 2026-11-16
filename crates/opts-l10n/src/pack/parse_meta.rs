//! Parsing and warnings of the pack metadata file.

use super::*;

pub(super) fn check_meta_warnings(meta: &PackMeta, text: &str, issues: &mut Vec<Issue>) {
    let line_of = |field: &str| interchange::line_of_json_key(text, field, 0);
    if meta.authors.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            "pack-json",
            FILE,
            line_of("authors"),
            None,
            "`authors` is empty; name the people (or tools) behind the translation",
        ));
    }
    if meta.license != EXPECTED_LICENSE {
        issues.push(Issue::new(
            Severity::Warning,
            "licence",
            FILE,
            line_of("license"),
            None,
            format!(
                "`license` is `{}`; contributed text is licensed as the project: `{EXPECTED_LICENSE}`",
                meta.license
            ),
        ));
    }
}

/// Reads `pack.json` text. Problems become issues; a pack with an error has no meta.
#[must_use]
pub fn parse_meta(text: &str, folder: &str) -> (Option<PackMeta>, Vec<Issue>) {
    let mut issues = Vec::new();
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            issues.push(Issue::new(
                Severity::Error,
                "pack-json",
                FILE,
                Some(e.line()),
                None,
                format!(
                    "not valid JSON ({e}); fix the syntax (commas between fields, double quotes around text). The pack is not loaded"
                ),
            ));
            return (None, issues);
        }
    };
    let Some(obj) = value.as_object() else {
        issues.push(Issue::new(
            Severity::Error,
            "pack-json",
            FILE,
            Some(1),
            None,
            "the top level must be an object `{ ... }`",
        ));
        return (None, issues);
    };
    let line_of = |field: &str| interchange::line_of_json_key(text, field, 0);
    let code = extract_text_field(obj, "code", true, &mut issues, line_of("code"));
    let name = extract_text_field(obj, "name", true, &mut issues, line_of("name"));
    let native_name = extract_text_field(
        obj,
        "native_name",
        true,
        &mut issues,
        line_of("native_name"),
    );
    let version = extract_text_field(obj, "version", true, &mut issues, line_of("version"));
    let license = extract_text_field(obj, "license", true, &mut issues, line_of("license"));
    let base_version = extract_text_field(
        obj,
        "base_version",
        false,
        &mut issues,
        line_of("base_version"),
    );
    let direction = match obj.get("direction").and_then(|d| d.as_str()) {
        None | Some("ltr") => Direction::Ltr,
        Some("rtl") => Direction::Rtl,
        Some(_) => {
            fail_field(
                &mut issues,
                "direction",
                line_of("direction"),
                "must be \"ltr\" or \"rtl\"",
            );
            Direction::Ltr
        }
    };
    let authors = parse_authors(obj, &mut issues, line_of("authors"));
    validate_code(&code, folder, &mut issues, line_of("code"));
    if issues.iter().any(|i| i.severity == Severity::Error) {
        return (None, issues);
    }
    let meta = PackMeta {
        code,
        name,
        native_name,
        direction,
        version,
        authors,
        license,
        base_version,
    };
    check_meta_warnings(&meta, text, &mut issues);
    (Some(meta), issues)
}
