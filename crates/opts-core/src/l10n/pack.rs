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

/// Reads `pack.json` text. Problems become issues; a pack with an error has no meta.
#[must_use]
pub fn parse_meta(text: &str, folder: &str) -> (Option<PackMeta>, Vec<Issue>) {
    const FILE: &str = "pack.json";
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
    let mut fail = |field: &str, why: &str| {
        issues.push(Issue::new(
            Severity::Error,
            "pack-json",
            FILE,
            line_of(field),
            None,
            format!("`{field}`: {why}"),
        ));
    };
    let mut text_field = |field: &str, required: bool| -> String {
        match obj.get(field) {
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
            Some(serde_json::Value::String(_)) | None if required => {
                fail(field, "required, a non-empty text");
                String::new()
            }
            Some(serde_json::Value::String(s)) => s.clone(),
            None => String::new(),
            Some(_) => {
                fail(field, "must be text");
                String::new()
            }
        }
    };
    let code = text_field("code", true);
    let name = text_field("name", true);
    let native_name = text_field("native_name", true);
    let version = text_field("version", true);
    let license = text_field("license", true);
    let base_version = text_field("base_version", false);
    let direction = match obj.get("direction").and_then(|d| d.as_str()) {
        None | Some("ltr") => Direction::Ltr,
        Some("rtl") => Direction::Rtl,
        Some(_) => {
            fail("direction", "must be \"ltr\" or \"rtl\"");
            Direction::Ltr
        }
    };
    let authors: Vec<String> = match obj.get("authors") {
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
        Some(_) => {
            fail(
                "authors",
                "must be a list of names, for example [\"Jane Doe\"]",
            );
            Vec::new()
        }
    };
    if !code.is_empty() {
        if !valid_code(&code) {
            fail(
                "code",
                "must look like `fr` or `pt-BR` (lower-case language, optional region)",
            );
        } else if code != folder {
            fail(
                "code",
                &format!("is `{code}` but the folder is `{folder}`; they must be equal"),
            );
        } else if code == "en-XA" || code.eq_ignore_ascii_case("en-xa") {
            fail(
                "code",
                "`en-XA` is the pseudo-locale of the tests and cannot be a pack",
            );
        }
    }
    if issues.iter().any(|i| i.severity == Severity::Error) {
        return (None, issues);
    }
    if authors.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            "pack-json",
            FILE,
            line_of("authors"),
            None,
            "`authors` is empty; name the people (or tools) behind the translation",
        ));
    }
    if license != EXPECTED_LICENSE {
        issues.push(Issue::new(
            Severity::Warning,
            "licence",
            FILE,
            line_of("license"),
            None,
            format!("`license` is `{license}`; contributed text is licensed as the project: `{EXPECTED_LICENSE}`"),
        ));
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
    (Some(meta), issues)
}

/// A pack read from disk: its identity, the messages that passed the checks, and every finding.
#[derive(Debug, Clone)]
pub struct Pack {
    /// The folder.
    pub dir: PathBuf,
    /// The folder name.
    pub folder: String,
    /// The identity; `None` when `pack.json` is missing or wrong (the pack is then unusable).
    pub meta: Option<PackMeta>,
    /// The accepted messages of all string files.
    pub table: Table,
    /// The string files read.
    pub files: Vec<String>,
    /// All findings.
    pub issues: Vec<Issue>,
    /// Completeness against the English reference.
    pub stats: Stats,
    /// Scripts the text needs that the bundled fonts lack.
    pub missing_fonts: BTreeSet<&'static str>,
    /// `true` when `base_version` differs from the current English revision.
    pub outdated: bool,
}

impl Pack {
    /// Number of errors.
    #[must_use]
    pub fn errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Number of warnings.
    #[must_use]
    pub fn warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// `true` when the pack has an identity and can be offered.
    #[must_use]
    pub fn usable(&self) -> bool {
        self.meta.is_some()
    }
}

fn read_text(path: &Path, shown: &str, issues: &mut Vec<Issue>) -> Option<String> {
    let fail = |issues: &mut Vec<Issue>, why: String| {
        issues.push(Issue::new(Severity::Error, "read", shown, None, None, why));
    };
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_FILE_BYTES => {
            fail(
                issues,
                format!(
                    "larger than {} KiB; the file is skipped",
                    MAX_FILE_BYTES / 1024
                ),
            );
            return None;
        }
        Ok(_) => {}
        Err(e) => {
            fail(issues, format!("cannot be read ({e}); the file is skipped"));
            return None;
        }
    }
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(t) => Some(t.strip_prefix('\u{feff}').map_or(t.clone(), str::to_string)),
            Err(e) => {
                fail(
                    issues,
                    format!("is not UTF-8 text (invalid byte at offset {}); save it as UTF-8. The file is skipped", e.utf8_error().valid_up_to()),
                );
                None
            }
        },
        Err(e) => {
            fail(issues, format!("cannot be read ({e}); the file is skipped"));
            None
        }
    }
}

/// Reads and checks the pack in `dir` against `reference`.
#[must_use]
pub fn load_pack_dir(dir: &Path, reference: &Reference) -> Pack {
    let folder = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut issues = Vec::new();
    let meta_path = dir.join("pack.json");
    let meta = if meta_path.exists() {
        read_text(&meta_path, "pack.json", &mut issues).and_then(|t| {
            let (m, i) = parse_meta(&t, &folder);
            issues.extend(i);
            m
        })
    } else {
        issues.push(Issue::new(
            Severity::Error,
            "pack-json",
            "pack.json",
            None,
            None,
            "missing; create it with `opts-tools l10n new <code>` or copy it from another pack. The folder is ignored",
        ));
        None
    };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            !n.starts_with('.') && (n.ends_with(".ftl") || n.ends_with(".json")) && n != "pack.json"
        })
        .collect();
    names.sort();
    if names.len() > MAX_FILES {
        issues.push(Issue::new(
            Severity::Warning,
            "read",
            ".",
            None,
            None,
            format!(
                "{} string files; only the first {MAX_FILES} are read",
                names.len()
            ),
        ));
        names.truncate(MAX_FILES);
    }
    let lang = meta
        .as_ref()
        .map_or(folder.as_str(), |m| m.code.as_str())
        .to_string();
    let mut table = Table::default();
    let mut origin: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut files = Vec::new();
    {
        for name in names {
            let Some(text) = read_text(&dir.join(&name), &name, &mut issues) else {
                continue;
            };
            files.push(name.clone());
            let parsed = if name.ends_with(".json") {
                let imported = interchange::parse_json(&text);
                if let (Some(l), Some(m)) = (&imported.language, &meta) {
                    if *l != m.code {
                        issues.push(Issue::new(
                            Severity::Warning,
                            "language-mismatch",
                            &name,
                            interchange::line_of_json_key(&text, "language", 0),
                            None,
                            format!("`language` is `{l}` but the pack is `{}`", m.code),
                        ));
                    }
                }
                imported.table
            } else {
                fluent::parse(&text)
            };
            let checked = check::validate(&parsed, &name, &lang, reference);
            issues.extend(checked.issues);
            for key in checked.table.order {
                if let Some(prev) = origin.insert(key.clone(), name.clone()) {
                    issues.push(Issue::new(
                        Severity::Warning,
                        "duplicate-key",
                        &name,
                        checked.table.lines.get(&key).copied(),
                        Some(&key),
                        format!("also defined in {prev}; this later definition wins"),
                    ));
                } else {
                    table.order.push(key.clone());
                }
                if let Some(p) = checked.table.messages.get(&key) {
                    table.messages.insert(key.clone(), p.clone());
                }
            }
        }
    }
    let (missing_fonts, rtl_text) = check::scripts_beyond_fonts(&table);
    if let Some(m) = &meta {
        if !missing_fonts.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                "font-coverage",
                "pack.json",
                None,
                None,
                format!(
                    "the text uses {} which the bundled fonts (Latin, Greek, Cyrillic) do not cover; those characters will show as empty boxes",
                    missing_fonts.iter().copied().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if m.direction == Direction::Rtl {
            issues.push(Issue::new(
                Severity::Warning,
                "rtl-unsupported",
                "pack.json",
                None,
                None,
                "right-to-left languages are not supported by the interface yet; the language is listed but cannot be selected",
            ));
        } else if rtl_text {
            issues.push(Issue::new(
                Severity::Warning,
                "direction",
                "pack.json",
                None,
                None,
                "the text is written right to left but `direction` is \"ltr\"; set \"rtl\" (not supported by the interface yet)",
            ));
        }
    }
    let revision = reference.revision();
    let outdated = meta
        .as_ref()
        .is_some_and(|m| !m.base_version.is_empty() && m.base_version != revision);
    if outdated {
        issues.push(Issue::new(
            Severity::Warning,
            "outdated",
            "pack.json",
            None,
            None,
            format!("made against English revision {} but the current one is {revision}; run `opts-tools l10n check` and translate the new or changed keys, then update `base_version`", meta.as_ref().map_or("", |m| m.base_version.as_str())),
        ));
    }
    let stats = check::stats(reference, &table, &lang);
    Pack {
        dir: dir.to_path_buf(),
        folder,
        meta,
        table,
        files,
        issues,
        stats,
        missing_fonts,
        outdated,
    }
}

/// Reads every pack folder directly below `root`. A missing `root` means no packs. Hidden folders
/// are skipped; at most [`MAX_PACKS`] are read.
#[must_use]
pub fn discover(root: &Path, reference: &Reference) -> Vec<Pack> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| !n.to_string_lossy().starts_with('.'))
        })
        .collect();
    dirs.sort();
    dirs.truncate(MAX_PACKS);
    dirs.iter().map(|d| load_pack_dir(d, reference)).collect()
}

/// The folder packs are read from: `<user data dir>/lang`.
#[must_use]
pub fn default_root() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("lang"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes() {
        for ok in ["fr", "pt-BR", "zh-Hans", "yue"] {
            assert!(valid_code(ok), "{ok}");
        }
        for bad in ["", "f", "FR", "fr_FR", "english", "../x", "fr-", "fr-x"] {
            assert!(!valid_code(bad), "{bad}");
        }
    }

    #[test]
    fn meta_errors_name_the_field_and_line() {
        let (m, issues) = parse_meta(
            "{\n  \"code\": \"fr\",\n  \"name\": \"French\",\n  \"native_name\": \"\",\n  \"version\": \"1\",\n  \"license\": \"MIT OR Apache-2.0\",\n  \"direction\": \"up\"\n}\n",
            "fr",
        );
        assert!(m.is_none());
        let at: Vec<(Option<usize>, &str)> = issues
            .iter()
            .map(|i| (i.line, i.message.as_str()))
            .collect();
        assert!(
            at.iter()
                .any(|(l, m)| *l == Some(4) && m.contains("native_name")),
            "{at:?}"
        );
        assert!(
            at.iter()
                .any(|(l, m)| *l == Some(7) && m.contains("direction")),
            "{at:?}"
        );
        let (_, broken) = parse_meta("{ \"code\": ", "fr");
        assert_eq!(broken[0].code, "pack-json");
    }

    #[test]
    fn code_must_equal_the_folder() {
        let text = "{\"code\":\"de\",\"name\":\"a\",\"native_name\":\"b\",\"version\":\"1\",\"license\":\"MIT OR Apache-2.0\",\"authors\":[\"x\"]}";
        assert!(parse_meta(text, "fr").0.is_none());
        assert!(parse_meta(text, "de").0.is_some());
    }

    #[test]
    fn meta_round_trips_through_its_json() {
        let m = super::super::builtin::meta_of("de").unwrap();
        let (back, issues) = parse_meta(&m.to_json(), "de");
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(back, Some(m));
    }
}
