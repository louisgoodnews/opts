//! The Fluent exporter and the import to Fluent.

use super::*;

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
