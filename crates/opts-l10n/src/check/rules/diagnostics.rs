//! Parse diagnostics and message-content validation of the rules.

use super::*;

pub(super) fn collect_parse_diagnostics(parsed: &Table, file: &str, out: &mut Checked) {
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

pub(super) fn validate_message_content(pattern: &Pattern) -> Result<(), (&'static str, String)> {
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
