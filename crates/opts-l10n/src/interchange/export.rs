//! Exporting reference and translation tables to JSON (`opts-l10n/1`) and `.ftl` templates.

use super::*;

fn q(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// The single `{ $var -> ... }` of a message that is only a plural selector.
fn as_plural(p: &Pattern) -> Option<(&str, &Vec<(String, Pattern)>)> {
    match p.as_slice() {
        [Part::Select { var, variants, .. }] => Some((var, variants)),
        _ => None,
    }
}

fn is_number_key(k: &str) -> bool {
    k.parse::<f64>().is_ok()
}

/// What an export contains.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExportOptions {
    /// Leave out the keys the translation already has.
    pub only_missing: bool,
}

/// The `$variables` (without `$`) of an English message, sorted.
fn variable_names(reference: &Reference, key: &str) -> Vec<String> {
    reference
        .variables_of(key)
        .map(|v| {
            v.iter()
                .filter_map(|s| s.strip_prefix('$').map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn group_of(reference: &Reference, key: &str) -> String {
    reference
        .file_of
        .get(key)
        .map(|f| f.trim_end_matches(".ftl").to_string())
        .unwrap_or_else(|| "strings".into())
}

fn format_plural_fields(
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

fn format_message_entry(
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

/// The JSON template for `lang`: every English message, the current translation from
/// `translation` (empty where missing), context and variables.
#[must_use]
pub fn export_json(
    reference: &Reference,
    translation: &Table,
    lang: &str,
    meta: &PackMeta,
    opts: ExportOptions,
) -> String {
    let lang_base = lang.split('-').next().unwrap_or(lang);
    let categories = fluent::plural_categories(lang_base);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"format\": {},\n", q(FORMAT)));
    out.push_str(&format!("  \"language\": {},\n", q(lang)));
    out.push_str(&format!(
        "  \"base_version\": {},\n",
        q(&reference.revision())
    ));
    let pack = meta.to_json();
    let pack_indented: Vec<String> = pack.trim_end().lines().map(|l| format!("  {l}")).collect();
    out.push_str(&format!(
        "  \"pack\": {},\n",
        pack_indented.join("\n").trim_start()
    ));
    out.push_str("  \"messages\": {");
    let mut first = true;
    for key in reference.keys() {
        if opts.only_missing && translation.messages.contains_key(key) {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        out.push('\n');
        out.push_str(&format_message_entry(
            key,
            reference,
            translation,
            categories,
        ));
    }
    out.push_str("\n  }\n}\n");
    out
}

/// The `.ftl` template for `lang`, one `(file name, text)` per English source file. Missing
/// messages carry the English text (so the file already works); translated ones carry the
/// translation and an `# English:` comment.
#[must_use]
pub fn export_ftl(
    reference: &Reference,
    translation: &Table,
    lang: &str,
    opts: ExportOptions,
) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for key in reference.keys() {
        if opts.only_missing && translation.messages.contains_key(key) {
            continue;
        }
        let name = reference
            .file_of
            .get(key)
            .cloned()
            .unwrap_or_else(|| "strings.ftl".into());
        let source = &reference.table.messages[key];
        let have = translation.messages.get(key);
        let mut comment = reference
            .table
            .comments
            .get(key)
            .cloned()
            .unwrap_or_default();
        let vars = variable_names(reference, key);
        if !vars.is_empty() {
            if !comment.is_empty() {
                comment.push('\n');
            }
            comment.push_str(&format!(
                "Variables (keep them): {}",
                vars.iter()
                    .map(|v| format!("$ {v}").replace("$ ", "$"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if have.is_some_and(|p| p != source) || lang == "en" {
            // translated already: show the source
        }
        if lang != "en" && have.is_some_and(|p| p != source) {
            let eng = fluent::to_source(source).replace('\n', " ");
            if !comment.is_empty() {
                comment.push('\n');
            }
            comment.push_str(&format!(
                "English: {}",
                eng.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
        let entry = fluent::to_ftl_entry(key, have.unwrap_or(source), &comment);
        match files.iter_mut().find(|(n, _)| *n == name) {
            Some((_, t)) => {
                t.push('\n');
                t.push_str(&entry);
            }
            None => files.push((
                name.clone(),
                format!(
                    "# {name}: messages for language `{lang}`. Translate the text after `=`; keep every key and every {{ $variable }}.\n\n{entry}"
                ),
            )),
        }
    }
    files
}

/// Turns the messages of an import into `.ftl` files `(file name, text)` grouped by their `file`
/// field (`strings.ftl` for entries without one), in the order of the interchange file.
#[must_use]
pub fn import_to_ftl(imported: &Imported) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for key in &imported.table.order {
        let group = imported
            .file_of
            .get(key)
            .map(|f| {
                f.chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect::<String>()
            })
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| "strings".into());
        let name = format!("{group}.ftl");
        let entry = fluent::to_ftl_entry(key, &imported.table.messages[key], "");
        match files.iter_mut().find(|(n, _)| *n == name) {
            Some((_, t)) => {
                t.push('\n');
                t.push_str(&entry);
            }
            None => files.push((name, entry)),
        }
    }
    files
}
