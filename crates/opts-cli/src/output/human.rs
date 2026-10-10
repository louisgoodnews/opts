//! Human-readable output.

use serde_json::Value;

use crate::protocol::RpcError;

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        other => other.to_string(),
    }
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Object(_) | Value::Array(_))
}

fn log_line(v: &Value) -> Option<String> {
    let o = v.as_object()?;
    Some(format!(
        "{} {:<5} {} {}",
        o.get("ts").map(scalar)?,
        o.get("level").map(scalar)?,
        o.get("target").map(scalar)?,
        o.get("message").map(scalar)?
    ))
}

fn write_value(out: &mut String, v: &Value, indent: usize) {
    let pad = "  ".repeat(indent);
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if is_scalar(val) {
                    out.push_str(&format!("{pad}{k}: {}\n", scalar(val)));
                } else {
                    out.push_str(&format!("{pad}{k}:\n"));
                    write_value(out, val, indent + 1);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                if let Some(line) = log_line(item) {
                    out.push_str(&format!("{pad}{line}\n"));
                } else if is_scalar(item) {
                    out.push_str(&format!("{pad}- {}\n", scalar(item)));
                } else if let Value::Object(map) = item {
                    let inline: Vec<String> = map
                        .iter()
                        .map(|(k, val)| {
                            let text = if is_scalar(val) {
                                scalar(val)
                            } else {
                                val.to_string()
                            };
                            format!("{k}={text}")
                        })
                        .collect();
                    out.push_str(&format!("{pad}- {}\n", inline.join("  ")));
                } else {
                    out.push_str(&format!("{pad}- {item}\n"));
                }
            }
        }
        other => out.push_str(&format!("{pad}{}\n", scalar(other))),
    }
}

/// Formats a result for a person: objects as `key: value` lines, lists one item per line.
#[must_use]
pub fn format_result(_method: &str, result: &Value) -> String {
    match result {
        Value::Null => "ok".into(),
        Value::Object(m) if m.len() == 1 && m.get("ok") == Some(&Value::Bool(true)) => "ok".into(),
        other => {
            let mut s = String::new();
            write_value(&mut s, other, 0);
            s.trim_end().to_string()
        }
    }
}

/// `error <code>: <message>` plus the details when present.
#[must_use]
pub fn format_error(error: &RpcError) -> String {
    let mut s = format!("error {}: {}", error.code, error.message);
    if let Some(data) = &error.data {
        s.push_str(&format!("\n  {data}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn objects_print_as_key_value_lines() {
        let s = format_result(
            "state",
            &json!({"app_state": "InGame", "fps": 60.5, "camera": {"mode": "free"}}),
        );
        assert!(s.contains("app_state: InGame"));
        assert!(s.contains("camera:\n  mode: free"));
    }

    #[test]
    fn ok_results_collapse() {
        assert_eq!(format_result("x", &Value::Null), "ok");
        assert_eq!(format_result("x", &json!({"ok": true})), "ok");
    }

    #[test]
    fn log_lines_are_one_per_row() {
        let v =
            json!({"lines": [{"ts": "10:00:00", "level": "WARN", "target": "t", "message": "m"}]});
        assert!(format_result("log.tail", &v).contains("10:00:00 WARN  t m"));
    }

    #[test]
    fn errors_show_code_and_data() {
        let e = RpcError::new(1004, "no such entity").with_data(json!({"id": 9}));
        assert_eq!(format_error(&e), "error 1004: no such entity\n  {\"id\":9}");
    }
}
