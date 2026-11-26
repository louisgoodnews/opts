//! Pattern, placeable and select parsing of the Fluent parser.

use super::*;

/// A pattern problem: what and, when known, the character offset.
pub(super) struct PatternError {
    pub(super) message: String,
    pub(super) offset: Option<usize>,
}

impl From<&str> for PatternError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
            offset: None,
        }
    }
}

pub(super) fn parse_pattern(text: &str) -> Result<Pattern, PatternError> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == LBRACE {
            let mut depth = 1;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                match chars[j] {
                    c if c == LBRACE => depth += 1,
                    c if c == RBRACE => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                return Err(PatternError {
                    message: "unclosed `{` (every opening brace needs a closing one)".into(),
                    offset: Some(i),
                });
            }
            if !buf.is_empty() {
                push_text(&mut parts, std::mem::take(&mut buf));
            }
            let inner: String = chars[i + 1..j - 1].iter().collect();
            match parse_placeable(inner.trim()).map_err(|mut e| {
                e.offset.get_or_insert(i);
                e
            })? {
                Part::Text(t) => push_text(&mut parts, t),
                other => parts.push(other),
            }
            i = j;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        push_text(&mut parts, buf);
    }
    Ok(parts)
}

/// Appends literal text, joining it to a literal before it.
pub(super) fn push_text(parts: &mut Pattern, text: String) {
    if let Some(Part::Text(last)) = parts.last_mut() {
        last.push_str(&text);
    } else {
        parts.push(Part::Text(text));
    }
}

/// Parses the text of one message value (the right-hand side of `key =`) on its own.
///
/// # Errors
/// A message and the character offset of the problem when known.
pub fn parse_value(text: &str) -> Result<Pattern, (String, Option<usize>)> {
    parse_pattern(text.trim()).map_err(|e| (e.message, e.offset))
}

pub(super) fn parse_placeable(inner: &str) -> Result<Part, PatternError> {
    if let Some((head, body)) = inner.split_once("->") {
        let var = head
            .trim()
            .strip_prefix('$')
            .ok_or("selector must be a $variable")?;
        return parse_select(var.trim(), body);
    }
    if let Some(var) = inner.strip_prefix('$') {
        return Ok(Part::Var(var.trim().to_string()));
    }
    if inner.len() >= 2 && inner.starts_with('"') && inner.ends_with('"') {
        return Ok(Part::Text(inner[1..inner.len() - 1].to_string()));
    }
    if inner
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && inner.chars().all(is_id_char)
    {
        return Ok(Part::Ref(inner.to_string()));
    }
    Err(PatternError {
        message: format!("unsupported placeable `{{ {inner} }}` (use `$variable`, a message id or a plural selector)"),
        offset: None,
    })
}

pub(super) fn parse_select(var: &str, body: &str) -> Result<Part, PatternError> {
    let mut variants: Vec<(String, String)> = Vec::new();
    let mut default = None;
    for line in body.lines() {
        let t = line.trim();
        let (star, rest) = match t.strip_prefix('*') {
            Some(r) => (true, r),
            None => (false, t),
        };
        if let Some(r) = rest.strip_prefix('[') {
            let (key, text) = r.split_once(']').ok_or("`[` without `]`")?;
            if star {
                default = Some(variants.len());
            }
            variants.push((key.trim().to_string(), text.trim().to_string()));
        } else if let Some(last) = variants.last_mut() {
            if !t.is_empty() {
                last.1.push('\n');
                last.1.push_str(t);
            }
        } else if !t.is_empty() {
            return Err("text before the first variant".into());
        }
    }
    let default = default.ok_or("selector needs a `*[default]` variant")?;
    let variants = variants
        .into_iter()
        .map(|(k, v)| parse_pattern(&v).map(|p| (k, p)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Part::Select {
        var: var.to_string(),
        variants,
        default,
    })
}
