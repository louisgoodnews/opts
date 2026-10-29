//! Language packs: a folder per language in the user data directory.
//!
//! ```text
//! <data>/lang/<code>/pack.json      identity of the pack (required)
//! <data>/lang/<code>/*.ftl          messages, Fluent subset (any file names)
//! <data>/lang/<code>/*.json         messages, interchange form (`opts-l10n/1` or a flat map)
//! ```
//!
//! Nothing here panics or returns an error to the caller: a pack that cannot be read or fails
//! its checks yields [`Issue`]s, and the messages that are fine are still used.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::check::{self, Issue, Reference, Severity, Stats};
use super::fluent::{self, Table};
use super::interchange;

mod load;
pub use load::{default_root, discover, load_pack_dir, Pack};

/// Largest string file read, in bytes.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Most packs discovered in one folder.
pub const MAX_PACKS: usize = 64;
/// Most string files read per pack.
pub const MAX_FILES: usize = 64;
/// The licence expected of contributed text.
pub const EXPECTED_LICENSE: &str = "MIT OR Apache-2.0";

/// Writing direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Left to right.
    #[default]
    Ltr,
    /// Right to left (not supported by the interface yet).
    Rtl,
}

impl Direction {
    /// `ltr` or `rtl`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

/// The identity of a pack (`pack.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackMeta {
    /// Language tag, also the folder name (`fr`, `pt-BR`).
    pub code: String,
    /// English name of the language.
    pub name: String,
    /// Name of the language in itself.
    pub native_name: String,
    /// Writing direction.
    pub direction: Direction,
    /// Version of the pack.
    pub version: String,
    /// Authors (names or handles); AI-assisted packs say so here.
    pub authors: Vec<String>,
    /// Licence of the text; expected [`EXPECTED_LICENSE`].
    pub license: String,
    /// The English revision ([`Reference::revision`]) the strings were made against; may be empty.
    pub base_version: String,
}

impl PackMeta {
    /// `pack.json` text (two-space indent, stable field order, trailing newline).
    #[must_use]
    pub fn to_json(&self) -> String {
        let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into());
        let authors: Vec<String> = self.authors.iter().map(|a| q(a)).collect();
        format!(
            "{{\n  \"code\": {},\n  \"name\": {},\n  \"native_name\": {},\n  \"direction\": {},\n  \"version\": {},\n  \"authors\": [{}],\n  \"license\": {},\n  \"base_version\": {}\n}}\n",
            q(&self.code),
            q(&self.name),
            q(&self.native_name),
            q(self.direction.name()),
            q(&self.version),
            authors.join(", "),
            q(&self.license),
            q(&self.base_version),
        )
    }
}

/// `true` for a well-formed language tag: a 2 or 3 letter lower-case language, then optional
/// 2 to 8 character alphanumeric subtags (`fr`, `pt-BR`, `zh-Hans`), at most 20 characters.
#[must_use]
pub fn valid_code(code: &str) -> bool {
    if code.len() > 20 {
        return false;
    }
    let mut parts = code.split('-');
    let lang = parts.next().unwrap_or("");
    (2..=3).contains(&lang.len())
        && lang.chars().all(|c| c.is_ascii_lowercase())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

const FILE: &str = "pack.json";

fn fail_field(issues: &mut Vec<Issue>, field: &str, line: Option<usize>, why: &str) {
    issues.push(Issue::new(
        Severity::Error,
        "pack-json",
        FILE,
        line,
        None,
        format!("`{field}`: {why}"),
    ));
}

fn extract_text_field(
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

fn parse_authors(
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

fn validate_code(code: &str, folder: &str, issues: &mut Vec<Issue>, line: Option<usize>) {
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

fn check_meta_warnings(meta: &PackMeta, text: &str, issues: &mut Vec<Issue>) {
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

#[cfg(test)]
mod tests;
