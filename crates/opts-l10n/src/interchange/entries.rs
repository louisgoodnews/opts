//! Reading the entries of a JSON interchange file.

use super::*;

pub(super) fn push_diag(table: &mut Table, line: usize, key: Option<&str>, message: impl Into<String>) {
    table.diagnostics.push(fluent::Diagnostic {
        line,
        key: key.map(str::to_string),
        message: message.into(),
    });
}

pub(super) fn parse_object_entry(
    o: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    out: &mut Imported,
) -> Result<Option<Pattern>, String> {
    if let Some(f) = o.get("file").and_then(|v| v.as_str()) {
        out.file_of.insert(key.to_string(), f.to_string());
    }
    if let Some(forms) = o.get("forms").and_then(|f| f.as_object()) {
        match o.get("variable").and_then(|v| v.as_str()) {
            Some(var) => match plural_pattern(var.trim_start_matches('$'), forms) {
                Ok(p) => Ok(Some(p)),
                Err(m) if m.is_empty() => Ok(None),
                Err(m) => Err(m),
            },
            None => Err("`forms` needs a `variable` (the name of the number, without `$`)".into()),
        }
    } else {
        match o.get("text") {
            Some(serde_json::Value::String(t)) => text_pattern(t),
            None => Ok(None),
            Some(_) => Err("`text` must be text".into()),
        }
    }
}

pub(super) fn parse_entry_value(
    entry: &serde_json::Value,
    key: &str,
    out: &mut Imported,
) -> Result<Option<Pattern>, String> {
    match entry {
        serde_json::Value::String(t) => text_pattern(t),
        serde_json::Value::Object(o) => parse_object_entry(o, key, out),
        _ => Err("an entry must be text or an object with `text` or `forms`".into()),
    }
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
    let Some((messages, base)) = extract_structured(top, text, &mut out) else {
        return out;
    };
    for (key, entry) in messages {
        let line = line_of_json_key(text, key, base).unwrap_or(1);
        match parse_entry_value(entry, key, &mut out) {
            Ok(Some(p)) => {
                out.table.messages.insert(key.to_string(), p);
                out.table.order.push(key.to_string());
                out.table.lines.insert(key.to_string(), line);
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
