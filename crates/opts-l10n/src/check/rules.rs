//! Validation rules for parsed localization tables against reference tables.

use super::*;

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
pub(super) fn is_prose(text: &str) -> bool {
    text.split_whitespace().count() >= 2 && text.chars().any(char::is_alphabetic)
}

pub(super) fn language_of(lang: &str) -> &str {
    lang.split(['-', '_']).next().unwrap_or(lang)
}

/// Whether `lang` writes without word spaces and is much shorter than English (no length checks).
fn is_compact_script(lang: &str) -> bool {
    matches!(language_of(lang), "ja" | "zh" | "ko")
}

pub(super) fn collect_texts(p: &Pattern, out: &mut String) {
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

fn collect_parse_diagnostics(parsed: &Table, file: &str, out: &mut Checked) {
    for d in &parsed.diagnostics {
        let dup = d.message.starts_with("duplicate key");
        out.issues.push(Issue::new(
            if dup {
                Severity::Warning
            } else {
                Severity::Error
            },
            if dup { "duplicate-key" } else { "syntax" },
            file,
            Some(d.line),
            d.key.as_deref(),
            if dup {
                d.message.clone()
            } else {
                format!(
                    "{}; the message is skipped and English shows instead",
                    d.message
                )
            },
        ));
    }
}

fn validate_message_content(pattern: &Pattern) -> Result<(), (&'static str, String)> {
    let mut text = String::new();
    collect_texts(pattern, &mut text);
    if text.trim().is_empty() && !pattern.iter().any(|p| !matches!(p, Part::Text(_))) {
        return Err((
            "empty",
            "the message is empty; delete the key to use the English text".into(),
        ));
    }
    if text.chars().count() > MAX_MESSAGE_CHARS {
        return Err((
            "too-long",
            format!("more than {MAX_MESSAGE_CHARS} characters"),
        ));
    }
    if let Some(c) = text
        .chars()
        .find(|c| (c.is_control() && *c != '\n') || *c == '\u{fffd}' || *c == '\u{27e6}')
    {
        return Err((
            "bad-character",
            format!(
                "contains the character U+{:04X}, which cannot be shown (control character, replacement character or the gap marker)",
                c as u32
            ),
        ));
    }
    Ok(())
}

fn validate_variables(
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

/// The key being checked and where it is: shared by the per-key warning checks.
struct KeyContext<'a> {
    key: &'a str,
    lang: &'a str,
    required: &'a [&'a str],
    line: Option<usize>,
    file: &'a str,
}

fn check_message_warnings(
    ctx: &KeyContext,
    pattern: &Pattern,
    source: &Pattern,
    out: &mut Checked,
) {
    let KeyContext {
        key,
        lang,
        required,
        line,
        file,
    } = *ctx;
    let (en_vis, tr_vis) = (visible_text(source), visible_text(pattern));
    let (en_len, tr_len) = (en_vis.chars().count(), tr_vis.chars().count());
    if !is_compact_script(lang) && tr_len as f32 > en_len as f32 * 2.5 + 12.0 {
        out.issues.push(Issue::new(
            Severity::Warning,
            "suspicious-length",
            file,
            line,
            Some(key),
            format!("{tr_len} characters against {en_len} in English; it may not fit its button or label"),
        ));
    }
    if language_of(lang) != "en" && pattern == source && is_prose(&en_vis) {
        out.issues.push(Issue::new(
            Severity::Warning,
            "untranslated-copy",
            file,
            line,
            Some(key),
            "identical to the English text; translate it or delete the key",
        ));
    }
    let mut sel = Vec::new();
    selects(pattern, &mut sel);
    for (var, variants) in sel {
        let missing: Vec<&&str> = required
            .iter()
            .filter(|c| !variants.iter().any(|v| v == **c))
            .collect();
        let has_default_other = variants.iter().any(|v| v == "other");
        if !missing.is_empty()
            && !(missing.len() == 1 && *missing[0] == "other" && !has_default_other)
        {
            out.issues.push(Issue::new(
                Severity::Warning,
                "plural-forms",
                file,
                line,
                Some(key),
                format!(
                    "`{{ ${var} -> ... }}` lacks the plural form(s) {} that {} uses (the `*[other]` form covers them, but may read wrong)",
                    missing.iter().map(|m| format!("[{m}]")).collect::<Vec<_>>().join(" "),
                    language_of(lang)
                ),
            ));
        }
    }
}

fn validate_single_key(
    key: &str,
    parsed: &Table,
    file: &str,
    lang: &str,
    reference: &Reference,
    required: &[&str],
    out: &mut Checked,
) {
    let pattern = &parsed.messages[key];
    let line = parsed.lines.get(key).copied();
    let Some(source) = reference.table.messages.get(key) else {
        out.issues.push(Issue::new(
            Severity::Error,
            "unknown-key",
            file,
            line,
            Some(key),
            "no such key in English; check the spelling (keys are lower-case words joined by dashes). The message is skipped",
        ));
        return;
    };
    if let Err((code, msg)) = validate_message_content(pattern) {
        out.issues.push(Issue::new(
            Severity::Error,
            code,
            file,
            line,
            Some(key),
            msg,
        ));
        return;
    }
    if let Err((code, msg)) = validate_variables(pattern, key, reference) {
        out.issues.push(Issue::new(
            Severity::Error,
            code,
            file,
            line,
            Some(key),
            msg,
        ));
        return;
    }
    let ctx = KeyContext {
        key,
        lang,
        required,
        line,
        file,
    };
    check_message_warnings(&ctx, pattern, source, out);
    out.table.messages.insert(key.to_string(), pattern.clone());
    out.table.order.push(key.to_string());
    if let Some(l) = line {
        out.table.lines.insert(key.to_string(), l);
    }
    if let Some(c) = parsed.comments.get(key) {
        out.table.comments.insert(key.to_string(), c.clone());
    }
}

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
