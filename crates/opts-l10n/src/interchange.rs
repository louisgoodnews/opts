//! The JSON interchange form (`opts-l10n/1`) and the `.ftl` template: what `opts-tools l10n
//! export` writes and `import` reads.
//!
//! The JSON form is lossless: every message of the Fluent subset (variables, references, plural
//! selectors, text mixed with selectors) survives `export` then `import` unchanged. It exists
//! because it is easy for scripts and AI agents to fill: one object per message with the English
//! source, the context and the variables next to the field to fill.
//!
//! ```json
//! {
//!   "format": "opts-l10n/1",
//!   "language": "fr",
//!   "base_version": "1a2b3c4d",
//!   "pack": { "code": "fr", "name": "French", "...": "..." },
//!   "messages": {
//!     "widget-clear": { "file": "gui", "source": "Clear", "text": "Effacer" },
//!     "widget-items": {
//!       "file": "gui", "variable": "count",
//!       "source_forms": { "one": "{ $count } item", "other": "{ $count } items" },
//!       "forms": { "one": "{ $count } élément", "many": "", "other": "{ $count } éléments" }
//!     }
//!   }
//! }
//! ```
//!
//! An empty `text` (or an empty set of `forms`) means *not translated yet* and is skipped without
//! complaint. A plain object `{ "key": "text", ... }` is accepted as well (flat form).

use std::collections::{BTreeMap, BTreeSet};

use super::check::Reference;
use super::fluent::{self, Part, Pattern, Table};
use super::pack::PackMeta;

mod export;
pub use export::{export_ftl, export_json, import_to_ftl, ExportOptions};

/// The value of `format` in JSON files.
pub const FORMAT: &str = "opts-l10n/1";

/// Result of reading an interchange file.
#[derive(Debug, Clone, Default)]
pub struct Imported {
    /// The messages (with `lines` and `diagnostics`), ready for [`super::check::validate`].
    pub table: Table,
    /// `language`, when given.
    pub language: Option<String>,
    /// `base_version`, when given.
    pub base_version: Option<String>,
    /// The `pack` object, when given.
    pub pack: Option<serde_json::Value>,
    /// Message key to its `file` group, when given.
    pub file_of: BTreeMap<String, String>,
}

/// The 1-based line of the first `"key"` followed by a colon at or after byte `from`.
#[must_use]
pub fn line_of_json_key(text: &str, key: &str, from: usize) -> Option<usize> {
    let needle = format!("\"{key}\"");
    let mut at = from.min(text.len());
    while let Some(i) = text.get(at..)?.find(&needle) {
        let pos = at + i;
        let after = text[pos + needle.len()..].trim_start();
        if after.starts_with(':') {
            return Some(text[..pos].matches('\n').count() + 1);
        }
        at = pos + needle.len();
    }
    None
}

fn push_diag(table: &mut Table, line: usize, key: Option<&str>, message: impl Into<String>) {
    table.diagnostics.push(fluent::Diagnostic {
        line,
        key: key.map(str::to_string),
        message: message.into(),
    });
}

fn plural_pattern(
    var: &str,
    forms: &serde_json::Map<String, serde_json::Value>,
) -> Result<Pattern, String> {
    let mut variants = Vec::new();
    let mut filled = 0;
    let mut empty = Vec::new();
    for (k, v) in forms {
        let t = v
            .as_str()
            .ok_or_else(|| format!("form `{k}` must be text"))?
            .trim();
        if t.is_empty() {
            empty.push(k.clone());
            continue;
        }
        filled += 1;
        let p = fluent::parse_value(t).map_err(|(m, _)| format!("form `{k}`: {m}"))?;
        variants.push((k.clone(), p));
    }
    if filled == 0 {
        return Err(String::new());
    }
    if !empty.is_empty() {
        return Err(format!(
            "form(s) {} are empty while others are filled; fill all forms or none (a form the language does not use may repeat another)",
            empty.iter().map(|e| format!("`{e}`")).collect::<Vec<_>>().join(", ")
        ));
    }
    let default = variants
        .iter()
        .position(|(k, _)| k == "other")
        .ok_or("forms need an `other` entry (the form used when no other matches)")?;
    Ok(vec![Part::Select {
        var: var.to_string(),
        variants,
        default,
    }])
}

/// Reads an interchange file. Problems are recorded in the table's diagnostics, never returned
/// as failures; entries that cannot be read are left out.
#[must_use]
pub fn parse_json(text: &str) -> Imported {
    let mut out = Imported::default();
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            push_diag(
                &mut out.table,
                e.line().max(1),
                None,
                format!("not valid JSON ({e}); fix the syntax (commas between entries, double quotes around text, `\\n` for line breaks)"),
            );
            return out;
        }
    };
    let Some(top) = value.as_object() else {
        push_diag(
            &mut out.table,
            1,
            None,
            "the top level must be an object `{ ... }`",
        );
        return out;
    };
    let structured = top.contains_key("messages") || top.contains_key("format");
    let (messages, base) = if structured {
        out.language = top
            .get("language")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        out.base_version = top
            .get("base_version")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        out.pack = top.get("pack").cloned();
        if let Some(f) = top.get("format").and_then(|v| v.as_str()) {
            if f != FORMAT {
                push_diag(
                    &mut out.table,
                    line_of_json_key(text, "format", 0).unwrap_or(1),
                    None,
                    format!("unknown format `{f}`; this version reads `{FORMAT}`"),
                );
            }
        }
        let Some(m) = top.get("messages").and_then(|m| m.as_object()) else {
            push_diag(
                &mut out.table,
                1,
                None,
                "`messages` must be an object of key to entry",
            );
            return out;
        };
        (m, text.find("\"messages\"").unwrap_or(0))
    } else {
        (top, 0)
    };
    for (key, entry) in messages {
        let line = line_of_json_key(text, key, base).unwrap_or(1);
        let result: Result<Option<Pattern>, String> = match entry {
            serde_json::Value::String(t) => text_pattern(t),
            serde_json::Value::Object(o) => {
                if let Some(f) = o.get("file").and_then(|v| v.as_str()) {
                    out.file_of.insert(key.clone(), f.to_string());
                }
                if let Some(forms) = o.get("forms").and_then(|f| f.as_object()) {
                    match o.get("variable").and_then(|v| v.as_str()) {
                        Some(var) => match plural_pattern(var.trim_start_matches('$'), forms) {
                            Ok(p) => Ok(Some(p)),
                            Err(m) if m.is_empty() => Ok(None),
                            Err(m) => Err(m),
                        },
                        None => Err(
                            "`forms` needs a `variable` (the name of the number, without `$`)"
                                .into(),
                        ),
                    }
                } else {
                    match o.get("text") {
                        Some(serde_json::Value::String(t)) => text_pattern(t),
                        None => Ok(None),
                        Some(_) => Err("`text` must be text".into()),
                    }
                }
            }
            _ => Err("an entry must be text or an object with `text` or `forms`".into()),
        };
        match result {
            Ok(Some(p)) => {
                out.table.messages.insert(key.clone(), p);
                out.table.order.push(key.clone());
                out.table.lines.insert(key.clone(), line);
            }
            Ok(None) => {}
            Err(m) => push_diag(&mut out.table, line, Some(key), m),
        }
    }
    out.table.errors = out
        .table
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .collect();
    out
}

fn text_pattern(t: &str) -> Result<Option<Pattern>, String> {
    if t.trim().is_empty() {
        return Ok(None);
    }
    fluent::parse_value(t).map(Some).map_err(|(m, _)| m)
}

#[cfg(test)]
mod tests;
