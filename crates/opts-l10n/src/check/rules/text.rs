//! Text extraction and selector walking of the validation rules.

use super::*;

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
pub(in crate::check) fn is_prose(text: &str) -> bool {
    text.split_whitespace().count() >= 2 && text.chars().any(char::is_alphabetic)
}

pub(in crate::check) fn language_of(lang: &str) -> &str {
    lang.split(['-', '_']).next().unwrap_or(lang)
}

/// Whether `lang` writes without word spaces and is much shorter than English (no length checks).
pub(super) fn is_compact_script(lang: &str) -> bool {
    matches!(language_of(lang), "ja" | "zh" | "ko")
}

pub(in crate::check) fn collect_texts(p: &Pattern, out: &mut String) {
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
pub(super) fn selects(p: &Pattern, out: &mut Vec<(String, Vec<String>)>) {
    for part in p {
        if let Part::Select { var, variants, .. } = part {
            out.push((
                var.clone(),
                variants.iter().map(|(k, _)| k.clone()).collect(),
            ));
            for (_, v) in variants {
                selects(v, out);
            }
        }
    }
}
