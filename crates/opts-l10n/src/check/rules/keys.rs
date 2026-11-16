//! Per-key checks: message warnings and the single-key validation.

use super::*;

/// The key being checked and where it is: shared by the per-key warning checks.
pub(super) struct KeyContext<'a> {
    key: &'a str,
    lang: &'a str,
    required: &'a [&'a str],
    line: Option<usize>,
    file: &'a str,
}

pub(super) fn check_message_warnings(
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

pub(super) fn validate_single_key(
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
