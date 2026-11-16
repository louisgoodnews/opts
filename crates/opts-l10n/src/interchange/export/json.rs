//! The JSON exporter.

use super::*;

/// What an export contains.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExportOptions {
    /// Leave out the keys the translation already has.
    pub only_missing: bool,
}

/// The `$variables` (without `$`) of an English message, sorted.
pub(super) fn variable_names(reference: &Reference, key: &str) -> Vec<String> {
    reference
        .variables_of(key)
        .map(|v| {
            v.iter()
                .filter_map(|s| s.strip_prefix('$').map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn group_of(reference: &Reference, key: &str) -> String {
    reference
        .file_of
        .get(key)
        .map(|f| f.trim_end_matches(".ftl").to_string())
        .unwrap_or_else(|| "strings".into())
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
