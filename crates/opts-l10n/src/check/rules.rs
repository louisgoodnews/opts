//! Validation rules for parsed localization tables against reference tables.

use super::*;

mod diagnostics;
mod keys;
mod text;
mod variables;

use diagnostics::*;
use keys::*;
pub use text::*;
use variables::*;

/// Longest accepted text, in characters, of one message.
pub const MAX_MESSAGE_CHARS: usize = 2000;

/// Validates the parsed `.ftl` (or JSON) table `parsed` of file `file` for language `lang`
/// against `reference`. Parse problems are included. Nothing here panics or aborts: every
/// problem becomes an [`Issue`] and the faulty message is dropped from the result.
#[must_use]
pub fn validate(parsed: &Table, file: &str, lang: &str, reference: &Reference) -> Checked {
    let mut out = Checked::default();
    collect_parse_diagnostics(parsed, file, &mut out);
    let mut keys: Vec<&String> = parsed.messages.keys().collect();
    keys.sort_by_key(|k| parsed.lines.get(*k).copied().unwrap_or(usize::MAX));
    let required: &[&str] = fluent::plural_categories(language_of(lang));
    for key in keys {
        validate_single_key(key, parsed, file, lang, reference, required, &mut out);
    }
    out
}
