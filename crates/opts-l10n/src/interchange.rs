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

mod entries;
mod structured;

pub use entries::*;
use structured::*;

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

#[cfg(test)]
mod tests;
