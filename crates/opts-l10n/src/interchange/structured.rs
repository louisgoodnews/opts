//! Structured JSON values: plural and text patterns.

use super::*;

pub(super) fn plural_pattern(
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

pub(super) fn extract_structured<'a>(
    top: &'a serde_json::Map<String, serde_json::Value>,
    text: &str,
    out: &mut Imported,
) -> Option<(&'a serde_json::Map<String, serde_json::Value>, usize)> {
    let structured = top.contains_key("messages") || top.contains_key("format");
    if !structured {
        return Some((top, 0));
    }
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
        return None;
    };
    Some((m, text.find("\"messages\"").unwrap_or(0)))
}

pub(super) fn text_pattern(t: &str) -> Result<Option<Pattern>, String> {
    if t.trim().is_empty() {
        return Ok(None);
    }
    fluent::parse_value(t).map(Some).map_err(|(m, _)| m)
}
