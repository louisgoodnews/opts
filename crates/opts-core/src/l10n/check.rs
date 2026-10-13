//! Checking a translation against the English source: the rules behind `opts-tools l10n check`
//! and the language list of the game.
//!
//! Errors make a message unusable: it is **skipped** and the English text shows instead. Warnings
//! are reported and the message is kept.

use std::collections::{BTreeSet, HashMap};

use super::fluent::{self, Part, Pattern, Table};

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Reported, message kept.
    Warning,
    /// Message (or pack) skipped.
    Error,
}

impl Severity {
    /// `error` or `warning`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One finding: where it is, what is wrong and what to do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Error or warning.
    pub severity: Severity,
    /// A stable identifier (`syntax`, `unknown-key`, `variable-mismatch`, `empty`, ...), listed in
    /// `docs/src/architecture/localisation-packs.md`.
    pub code: &'static str,
    /// The file, relative to the pack folder (or the name the caller gave).
    pub file: String,
    /// 1-based line when known.
    pub line: Option<usize>,
    /// The message key concerned.
    pub key: Option<String>,
    /// What is wrong and how to fix it.
    pub message: String,
}

impl Issue {
    /// A new issue.
    #[must_use]
    pub fn new(
        severity: Severity,
        code: &'static str,
        file: &str,
        line: Option<usize>,
        key: Option<&str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            file: file.to_string(),
            line,
            key: key.map(str::to_string),
            message: message.into(),
        }
    }

    /// The JSON form (`severity`, `code`, `file`, `line`, `key`, `message`).
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "severity": self.severity.name(),
            "code": self.code,
            "file": self.file,
            "line": self.line,
            "key": self.key,
            "message": self.message,
        })
    }
}

impl std::fmt::Display for Issue {
    /// `file:line: error[code]: message (key)`, the shape compilers use, so editors can jump to it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}: ", self.file)?,
            None => write!(f, "{}: ", self.file)?,
        }
        write!(f, "{}[{}]: {}", self.severity.name(), self.code, self.message)?;
        if let Some(k) = &self.key {
            write!(f, " (key `{k}`)")?;
        }
        Ok(())
    }
}

/// The English source every translation is checked against.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    /// All English messages (comments and file order included).
    pub table: Table,
    /// Message key to the name of the source file it comes from.
    pub file_of: HashMap<String, String>,
    vars: HashMap<String, BTreeSet<String>>,
}

impl Reference {
    /// A reference from named `.ftl` sources `(file name, text)`; later files may not redefine a
    /// key of an earlier one (the later definition wins).
    #[must_use]
    pub fn from_sources(sources: &[(&str, &str)]) -> Self {
        let mut table = Table::default();
        let mut file_of = HashMap::new();
        for (name, text) in sources {
            let t = fluent::parse(text);
            for key in &t.order {
                file_of.insert(key.clone(), (*name).to_string());
                if !table.messages.contains_key(key) {
                    table.order.push(key.clone());
                }
            }
            table.messages.extend(t.messages);
            table.lines.extend(t.lines);
            table.comments.extend(t.comments);
            table.diagnostics.extend(t.diagnostics);
        }
        table.errors = table.diagnostics.iter().map(ToString::to_string).collect();
        Self::from_table(table, file_of)
    }

    /// A reference over an already merged table.
    #[must_use]
    pub fn from_table(table: Table, file_of: HashMap<String, String>) -> Self {
        let vars = table
            .messages
            .iter()
            .map(|(k, p)| (k.clone(), fluent::variables(p)))
            .collect();
        Self {
            table,
            file_of,
            vars,
        }
    }

    /// Number of messages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.messages.len()
    }

    /// `true` without messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.messages.is_empty()
    }

    /// The keys in file order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.table.order
    }

    /// The variables and references of the English message `key`.
    #[must_use]
    pub fn variables_of(&self, key: &str) -> Option<&BTreeSet<String>> {
        self.vars.get(key)
    }

    /// A short hash of all English keys and texts: a pack records it as `base_version`, so a later
    /// check can tell that the source has moved on.
    #[must_use]
    pub fn revision(&self) -> String {
        let mut keys: Vec<&String> = self.table.messages.keys().collect();
        keys.sort();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for k in keys {
            let line = format!("{k}={}\n", fluent::to_source(&self.table.messages[k]));
            for b in line.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{:08x}", h >> 32 ^ (h & 0xffff_ffff))
    }
}

/// A table after validation: only the messages that passed, and the findings.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    /// The accepted messages (`messages`, `lines`, `order`, `comments` are filled).
    pub table: Table,
    /// All findings.
    pub issues: Vec<Issue>,
}

/// Longest accepted text, in characters, of one message.
pub const MAX_MESSAGE_CHARS: usize = 2000;

/// The text of a pattern as a reader sees it, without placeholders (a select shows its default).
#[must_use]
pub fn visible_text(pattern: &Pattern) -> String {
    let mut out = String::new();
    for part in pattern {
        match part {
            Part::Text(t) => out.push_str(t),
            Part::Select {
                variants, default, ..
            } => {
                if let Some((_, p)) = variants.get(*default) {
                    out.push_str(&visible_text(p));
                }
            }
            Part::Var(_) | Part::Ref(_) => {}
        }
    }
    out
}

/// `true` when a text is prose (two words or more with letters): identical English and
/// translation then mean "not translated"; names, symbols and one-word loanwords may be equal.
fn is_prose(text: &str) -> bool {
    text.split_whitespace().count() >= 2 && text.chars().any(char::is_alphabetic)
}

fn language_of(lang: &str) -> &str {
    lang.split(['-', '_']).next().unwrap_or(lang)
}

/// Whether `lang` writes without word spaces and is much shorter than English (no length checks).
fn is_compact_script(lang: &str) -> bool {
    matches!(language_of(lang), "ja" | "zh" | "ko")
}

fn collect_texts(p: &Pattern, out: &mut String) {
    for part in p {
        match part {
            Part::Text(t) => out.push_str(t),
            Part::Select { variants, .. } => {
                for (_, v) in variants {
                    collect_texts(v, out);
                }
            }
            _ => {}
        }
    }
}

/// Plural variants of the selects of `pattern`: `(variable, variant keys)` each.
fn selects(p: &Pattern, out: &mut Vec<(String, Vec<String>)>) {
    for part in p {
        if let Part::Select { var, variants, .. } = part {
            out.push((var.clone(), variants.iter().map(|(k, _)| k.clone()).collect()));
            for (_, v) in variants {
                selects(v, out);
            }
        }
    }
}

/// Validates the parsed `.ftl` (or JSON) table `parsed` of file `file` for language `lang`
/// against `reference`. Parse problems are included. Nothing here panics or aborts: every
/// problem becomes an [`Issue`] and the faulty message is dropped from the result.
#[must_use]
pub fn validate(parsed: &Table, file: &str, lang: &str, reference: &Reference) -> Checked {
    let mut out = Checked::default();
    for d in &parsed.diagnostics {
        let dup = d.message.starts_with("duplicate key");
        out.issues.push(Issue::new(
            if dup { Severity::Warning } else { Severity::Error },
            if dup { "duplicate-key" } else { "syntax" },
            file,
            Some(d.line),
            d.key.as_deref(),
            if dup {
                d.message.clone()
            } else {
                format!("{}; the message is skipped and English shows instead", d.message)
            },
        ));
    }
    let mut keys: Vec<&String> = parsed.messages.keys().collect();
    keys.sort_by_key(|k| parsed.lines.get(*k).copied().unwrap_or(usize::MAX));
    let required: &[&str] = fluent::plural_categories(language_of(lang));
    for key in keys {
        let pattern = &parsed.messages[key];
        let line = parsed.lines.get(key).copied();
        let mut bad = |code: &'static str, message: String| {
            out.issues
                .push(Issue::new(Severity::Error, code, file, line, Some(key), message));
        };
        let Some(source) = reference.table.messages.get(key) else {
            bad(
                "unknown-key",
                "no such key in English; check the spelling (keys are lower-case words joined by dashes). The message is skipped".into(),
            );
            continue;
        };
        let mut text = String::new();
        collect_texts(pattern, &mut text);
        if text.trim().is_empty() && !pattern.iter().any(|p| !matches!(p, Part::Text(_))) {
            bad(
                "empty",
                "the message is empty; delete the key to use the English text".into(),
            );
            continue;
        }
        if text.chars().count() > MAX_MESSAGE_CHARS {
            bad(
                "too-long",
                format!("more than {MAX_MESSAGE_CHARS} characters"),
            );
            continue;
        }
        if let Some(c) = text
            .chars()
            .find(|c| (c.is_control() && *c != '\n') || *c == '\u{fffd}' || *c == '\u{27e6}')
        {
            bad(
                "bad-character",
                format!(
                    "contains the character U+{:04X}, which cannot be shown (control character, replacement character or the gap marker)",
                    c as u32
                ),
            );
            continue;
        }
        let have = fluent::variables(pattern);
        let want = reference.variables_of(key).cloned().unwrap_or_default();
        let is_var = |s: &&String| s.starts_with('$');
        let lost: Vec<&String> = want.iter().filter(is_var).filter(|v| !have.contains(*v)).collect();
        let added: Vec<&String> = have.iter().filter(is_var).filter(|v| !want.contains(*v)).collect();
        if !lost.is_empty() || !added.is_empty() {
            let mut why = String::new();
            if !lost.is_empty() {
                why.push_str(&format!(
                    "missing {}; ",
                    lost.iter().map(|s| format!("`{{ {s} }}`")).collect::<Vec<_>>().join(", ")
                ));
            }
            if !added.is_empty() {
                why.push_str(&format!(
                    "unknown {}; ",
                    added.iter().map(|s| format!("`{{ {s} }}`")).collect::<Vec<_>>().join(", ")
                ));
            }
            bad(
                "variable-mismatch",
                format!(
                    "the variables differ from English ({why}English uses {}). Keep every `{{ $name }}` exactly as written",
                    if want.is_empty() {
                        "none".to_string()
                    } else {
                        want.iter().filter(is_var).cloned().collect::<Vec<_>>().join(", ")
                    }
                ),
            );
            continue;
        }
        let unknown_refs: Vec<&String> = have
            .iter()
            .filter(|s| !s.starts_with('$') && !reference.table.messages.contains_key(*s))
            .collect();
        if !unknown_refs.is_empty() {
            bad(
                "unknown-reference",
                format!("refers to message(s) that do not exist: {unknown_refs:?}"),
            );
            continue;
        }
        // Accepted; warnings from here on.
        let mut warn = |code: &'static str, message: String| {
            out.issues.push(Issue::new(
                Severity::Warning,
                code,
                file,
                line,
                Some(key),
                message,
            ));
        };
        let (en_vis, tr_vis) = (visible_text(source), visible_text(pattern));
        let (en_len, tr_len) = (en_vis.chars().count(), tr_vis.chars().count());
        if !is_compact_script(lang) && tr_len as f32 > en_len as f32 * 2.5 + 12.0 {
            warn(
                "suspicious-length",
                format!("{tr_len} characters against {en_len} in English; it may not fit its button or label"),
            );
        }
        if language_of(lang) != "en" && pattern == source && is_prose(&en_vis) {
            warn(
                "untranslated-copy",
                "identical to the English text; translate it or delete the key".into(),
            );
        }
        let mut sel = Vec::new();
        selects(pattern, &mut sel);
        for (var, variants) in sel {
            let missing: Vec<&&str> = required
                .iter()
                .filter(|c| !variants.iter().any(|v| v == **c))
                .collect();
            let has_default_other = variants.iter().any(|v| v == "other");
            if !missing.is_empty() && !(missing.len() == 1 && *missing[0] == "other" && !has_default_other) {
                warn(
                    "plural-forms",
                    format!(
                        "`{{ ${var} -> ... }}` lacks the plural form(s) {} that {} uses (the `*[other]` form covers them, but may read wrong)",
                        missing.iter().map(|m| format!("[{m}]")).collect::<Vec<_>>().join(" "),
                        language_of(lang)
                    ),
                );
            }
        }
        out.table.messages.insert(key.clone(), pattern.clone());
        out.table.order.push(key.clone());
        if let Some(l) = line {
            out.table.lines.insert(key.clone(), l);
        }
        if let Some(c) = parsed.comments.get(key) {
            out.table.comments.insert(key.clone(), c.clone());
        }
    }
    out
}

/// How complete a translation is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    /// Number of English messages.
    pub total: usize,
    /// English messages the table has (valid ones).
    pub present: usize,
    /// Present and really translated (not an untouched copy of the English prose).
    pub translated: usize,
    /// Present but identical to English prose: probably untouched scaffold text.
    pub copied: usize,
    /// English keys the table lacks (English shows instead).
    pub missing: Vec<String>,
    /// Keys the table has that English lacks (ignored).
    pub unknown: Vec<String>,
}

impl Stats {
    /// Percentage of English messages that are translated, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            self.translated as f64 * 100.0 / self.total as f64
        }
    }
}

/// Completeness of `table` against `reference` (for language `lang`; English counts copies as
/// translated).
#[must_use]
pub fn stats(reference: &Reference, table: &Table, lang: &str) -> Stats {
    let mut s = Stats {
        total: reference.len(),
        ..Stats::default()
    };
    let english = language_of(lang) == "en";
    for key in reference.keys() {
        match table.messages.get(key) {
            None => s.missing.push(key.clone()),
            Some(p) => {
                s.present += 1;
                let same = reference.table.messages.get(key) == Some(p);
                if !english && same && is_prose(&visible_text(p)) {
                    s.copied += 1;
                } else {
                    s.translated += 1;
                }
            }
        }
    }
    let mut unknown: Vec<String> = table
        .messages
        .keys()
        .filter(|k| !reference.table.messages.contains_key(*k))
        .cloned()
        .collect();
    unknown.sort();
    s.unknown = unknown;
    s
}

/// Scripts a table's text needs that the bundled fonts (Noto Sans and DejaVu Sans: Latin, Greek,
/// Cyrillic) do not cover, and whether any is written right to left.
#[must_use]
pub fn scripts_beyond_fonts(table: &Table) -> (BTreeSet<&'static str>, bool) {
    let mut text = String::new();
    for p in table.messages.values() {
        collect_texts(p, &mut text);
    }
    let mut scripts = BTreeSet::new();
    let mut rtl = false;
    for c in text.chars() {
        let u = c as u32;
        let name = match u {
            0x0590..=0x05FF => Some(("Hebrew", true)),
            0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
                Some(("Arabic", true))
            }
            0x0700..=0x074F => Some(("Syriac", true)),
            0x0530..=0x058F => Some(("Armenian", false)),
            0x0900..=0x097F => Some(("Devanagari", false)),
            0x0980..=0x09FF => Some(("Bengali", false)),
            0x0B80..=0x0BFF => Some(("Tamil", false)),
            0x0E00..=0x0E7F => Some(("Thai", false)),
            0x10A0..=0x10FF => Some(("Georgian", false)),
            0x1100..=0x11FF | 0xAC00..=0xD7AF => Some(("Hangul", false)),
            0x1200..=0x137F => Some(("Ethiopic", false)),
            0x1780..=0x17FF => Some(("Khmer", false)),
            0x3040..=0x30FF => Some(("Kana", false)),
            0x3400..=0x4DBF | 0x4E00..=0x9FFF => Some(("CJK ideographs", false)),
            _ => None,
        };
        if let Some((n, r)) = name {
            scripts.insert(n);
            rtl |= r;
        }
    }
    (scripts, rtl)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> Reference {
        Reference::from_sources(&[(
            "en.ftl",
            "# Say hi.\nhello = Hi { $who }\nitems =\n    { $count ->\n        [one] { $count } item\n       *[other] { $count } items\n    }\nplain = Save the file now\n",
        )])
    }

    fn check(src: &str, lang: &str) -> Checked {
        validate(&fluent::parse(src), "t.ftl", lang, &reference())
    }

    fn codes(c: &Checked) -> Vec<&'static str> {
        c.issues.iter().map(|i| i.code).collect()
    }

    #[test]
    fn a_good_translation_passes_clean() {
        let c = check("hello = Salut { $who }\nplain = Enregistre le fichier maintenant\n", "fr");
        assert!(c.issues.is_empty(), "{:?}", c.issues);
        assert_eq!(c.table.messages.len(), 2);
    }

    #[test]
    fn unknown_empty_and_mismatched_messages_are_skipped_with_their_line() {
        let c = check(
            "hello = Salut\nnope = x\nplain =\nitems = x\n",
            "fr",
        );
        let found: Vec<(&str, Option<usize>)> = c.issues.iter().map(|i| (i.code, i.line)).collect();
        assert!(found.contains(&("variable-mismatch", Some(1))), "{found:?}");
        assert!(found.contains(&("unknown-key", Some(2))), "{found:?}");
        assert!(found.contains(&("empty", Some(3))), "{found:?}");
        assert!(c.table.messages.is_empty(), "{:?}", c.table.messages.keys());
    }

    #[test]
    fn copies_and_lengths_and_plural_forms_are_warnings() {
        let c = check("plain = Save the file now\nhello = Salut { $who }\nitems =\n    { $count ->\n       *[other] { $count } x\n    }\n", "ru");
        assert!(codes(&c).contains(&"untranslated-copy"), "{:?}", c.issues);
        assert!(codes(&c).contains(&"plural-forms"), "{:?}", c.issues);
        assert_eq!(c.table.messages.len(), 3, "warnings keep the message");
    }

    #[test]
    fn stats_count_missing_and_copies() {
        let r = reference();
        let c = check("plain = Save the file now\nhello = Salut { $who }\n", "fr");
        let s = stats(&r, &c.table, "fr");
        assert_eq!((s.total, s.present, s.translated, s.copied), (3, 2, 1, 1));
        assert_eq!(s.missing, vec!["items".to_string()]);
    }

    #[test]
    fn issue_display_is_editor_clickable() {
        let c = check("hello = Salut\n", "fr");
        let text = c.issues[0].to_string();
        assert!(text.starts_with("t.ftl:1: error[variable-mismatch]:"), "{text}");
    }

    #[test]
    fn revision_changes_with_the_source() {
        let a = reference().revision();
        let b = Reference::from_sources(&[("e.ftl", "hello = Hi\n")]).revision();
        assert_ne!(a, b);
        assert_eq!(a.len(), 8);
    }
}
