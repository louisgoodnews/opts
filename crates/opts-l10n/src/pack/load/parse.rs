//! Parsing and merging the message files of a pack.

use super::*;

pub(super) fn parse_pack_file(
    name: &str,
    text: &str,
    meta: Option<&PackMeta>,
    issues: &mut Vec<Issue>,
) -> Table {
    if name.ends_with(".json") {
        let imported = interchange::parse_json(text);
        if let (Some(l), Some(m)) = (&imported.language, meta) {
            if *l != m.code {
                issues.push(Issue::new(
                    Severity::Warning,
                    "language-mismatch",
                    name,
                    interchange::line_of_json_key(text, "language", 0),
                    None,
                    format!("`language` is `{l}` but the pack is `{}`", m.code),
                ));
            }
        }
        imported.table
    } else {
        fluent::parse(text)
    }
}

pub(super) fn merge_file_messages(
    name: &str,
    checked: check::Checked,
    origin: &mut std::collections::HashMap<String, String>,
    table: &mut Table,
    issues: &mut Vec<Issue>,
) {
    issues.extend(checked.issues);
    for key in checked.table.order {
        if let Some(prev) = origin.insert(key.clone(), name.to_string()) {
            issues.push(Issue::new(
                Severity::Warning,
                "duplicate-key",
                name,
                checked.table.lines.get(&key).copied(),
                Some(&key),
                format!("also defined in {prev}; this later definition wins"),
            ));
        } else {
            table.order.push(key.clone());
        }
        if let Some(p) = checked.table.messages.get(&key) {
            table.messages.insert(key.clone(), p.clone());
        }
    }
}
