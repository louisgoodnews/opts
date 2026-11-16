//! Shared formatting helpers of the exporters.

use super::*;

pub(super) fn q(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// The single `{ $var -> ... }` of a message that is only a plural selector.
pub(super) fn as_plural(p: &Pattern) -> Option<(&str, &Vec<(String, Pattern)>)> {
    match p.as_slice() {
        [Part::Select { var, variants, .. }] => Some((var, variants)),
        _ => None,
    }
}

pub(super) fn is_number_key(k: &str) -> bool {
    k.parse::<f64>().is_ok()
}

pub(super) fn format_plural_fields(
    source: &Pattern,
    have: Option<&Pattern>,
    categories: &[&str],
    fields: &mut Vec<String>,
) {
    let Some((var, variants)) = as_plural(source) else {
        fields.push(format!(
            "      \"source\": {}",
            q(&fluent::to_source(source))
        ));
        let text = have.map(fluent::to_source).unwrap_or_default();
        fields.push(format!("      \"text\": {}", q(&text)));
        return;
    };

    fields.push(format!("      \"variable\": {}", q(var)));
    let forms: Vec<String> = variants
        .iter()
        .map(|(k, p)| format!("{}: {}", q(k), q(&fluent::to_source(p))))
        .collect();
    fields.push(format!(
        "      \"source_forms\": {{ {} }}",
        forms.join(", ")
    ));
    let mut keys: Vec<String> = variants
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| is_number_key(k))
        .collect();
    keys.extend(categories.iter().map(|c| (*c).to_string()));
    let have_forms: BTreeMap<String, String> = have
        .and_then(|p| as_plural(p))
        .map(|(_, vs)| {
            vs.iter()
                .map(|(k, p)| (k.clone(), fluent::to_source(p)))
                .collect()
        })
        .unwrap_or_default();
    for k in have_forms.keys() {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    let mut seen = BTreeSet::new();
    keys.retain(|k| seen.insert(k.clone()));
    let filled: Vec<String> = keys
        .iter()
        .map(|k| {
            format!(
                "{}: {}",
                q(k),
                q(have_forms.get(k).map_or("", String::as_str))
            )
        })
        .collect();
    fields.push(format!("      \"forms\": {{ {} }}", filled.join(", ")));
}

pub(super) fn format_message_entry(
    key: &str,
    reference: &Reference,
    translation: &Table,
    categories: &[&str],
) -> String {
    let source = &reference.table.messages[key];
    let have = translation.messages.get(key);
    let mut fields: Vec<String> = Vec::new();
    if let Some(c) = reference.table.comments.get(key) {
        fields.push(format!("      \"context\": {}", q(c)));
    }
    fields.push(format!("      \"file\": {}", q(&group_of(reference, key))));
    let vars = variable_names(reference, key);
    fields.push(format!(
        "      \"variables\": [{}]",
        vars.iter().map(|v| q(v)).collect::<Vec<_>>().join(", ")
    ));
    format_plural_fields(source, have, categories, &mut fields);
    format!("    {}: {{\n{}\n    }}", q(key), fields.join(",\n"))
}
