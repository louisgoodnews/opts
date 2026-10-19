//! The JSON interchange form (`opts-l10n/1`) and the `.ftl` template: what `opts-tools l10n
//! export` writes and `import` reads.
//!
//! The JSON form is lossless: every message of the Fluent subset (variables, references, plural
//! selectors, text mixed with selectors) survives `export` then `import` unchanged. It exists
//! because it is easy for scripts and AI agents to fill: one object per message with the English
//! source, the context and the variables next to the field to fill.
//!
//! ```json
//! {
//!   "format": "opts-l10n/1",
//!   "language": "fr",
//!   "base_version": "1a2b3c4d",
//!   "pack": { "code": "fr", "name": "French", "...": "..." },
//!   "messages": {
//!     "widget-clear": { "file": "gui", "source": "Clear", "text": "Effacer" },
//!     "widget-items": {
//!       "file": "gui", "variable": "count",
//!       "source_forms": { "one": "{ $count } item", "other": "{ $count } items" },
//!       "forms": { "one": "{ $count } élément", "many": "", "other": "{ $count } éléments" }
//!     }
//!   }
//! }
//! ```
//!
//! An empty `text` (or an empty set of `forms`) means *not translated yet* and is skipped without
//! complaint. A plain object `{ "key": "text", ... }` is accepted as well (flat form).

use std::collections::{BTreeMap, BTreeSet};

use super::check::Reference;
use super::fluent::{self, Part, Pattern, Table};
use super::pack::PackMeta;

/// The value of `format` in JSON files.
pub const FORMAT: &str = "opts-l10n/1";

/// Result of reading an interchange file.
#[derive(Debug, Clone, Default)]
pub struct Imported {
    /// The messages (with `lines` and `diagnostics`), ready for [`super::check::validate`].
    pub table: Table,
    /// `language`, when given.
    pub language: Option<String>,
    /// `base_version`, when given.
    pub base_version: Option<String>,
    /// The `pack` object, when given.
    pub pack: Option<serde_json::Value>,
    /// Message key to its `file` group, when given.
    pub file_of: BTreeMap<String, String>,
}

/// The 1-based line of the first `"key"` followed by a colon at or after byte `from`.
#[must_use]
pub fn line_of_json_key(text: &str, key: &str, from: usize) -> Option<usize> {
    let needle = format!("\"{key}\"");
    let mut at = from.min(text.len());
    while let Some(i) = text.get(at..)?.find(&needle) {
        let pos = at + i;
        let after = text[pos + needle.len()..].trim_start();
        if after.starts_with(':') {
            return Some(text[..pos].matches('\n').count() + 1);
        }
        at = pos + needle.len();
    }
    None
}

fn push_diag(table: &mut Table, line: usize, key: Option<&str>, message: impl Into<String>) {
    table.diagnostics.push(fluent::Diagnostic {
        line,
        key: key.map(str::to_string),
        message: message.into(),
    });
}

fn plural_pattern(
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
    let structured = top.contains_key("messages") || top.contains_key("format");
    let (messages, base) = if structured {
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
            return out;
        };
        (m, text.find("\"messages\"").unwrap_or(0))
    } else {
        (top, 0)
    };
    for (key, entry) in messages {
        let line = line_of_json_key(text, key, base).unwrap_or(1);
        let result: Result<Option<Pattern>, String> = match entry {
            serde_json::Value::String(t) => text_pattern(t),
            serde_json::Value::Object(o) => {
                if let Some(f) = o.get("file").and_then(|v| v.as_str()) {
                    out.file_of.insert(key.clone(), f.to_string());
                }
                if let Some(forms) = o.get("forms").and_then(|f| f.as_object()) {
                    match o.get("variable").and_then(|v| v.as_str()) {
                        Some(var) => match plural_pattern(var.trim_start_matches('$'), forms) {
                            Ok(p) => Ok(Some(p)),
                            Err(m) if m.is_empty() => Ok(None),
                            Err(m) => Err(m),
                        },
                        None => Err(
                            "`forms` needs a `variable` (the name of the number, without `$`)"
                                .into(),
                        ),
                    }
                } else {
                    match o.get("text") {
                        Some(serde_json::Value::String(t)) => text_pattern(t),
                        None => Ok(None),
                        Some(_) => Err("`text` must be text".into()),
                    }
                }
            }
            _ => Err("an entry must be text or an object with `text` or `forms`".into()),
        };
        match result {
            Ok(Some(p)) => {
                out.table.messages.insert(key.clone(), p);
                out.table.order.push(key.clone());
                out.table.lines.insert(key.clone(), line);
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

fn text_pattern(t: &str) -> Result<Option<Pattern>, String> {
    if t.trim().is_empty() {
        return Ok(None);
    }
    fluent::parse_value(t).map(Some).map_err(|(m, _)| m)
}

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
        let source = &reference.table.messages[key];
        let have = translation.messages.get(key);
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("\n    {}: {{\n", q(key)));
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
        if let Some((var, variants)) = as_plural(source) {
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
        } else {
            fields.push(format!(
                "      \"source\": {}",
                q(&fluent::to_source(source))
            ));
            let text = have.map(fluent::to_source).unwrap_or_default();
            fields.push(format!("      \"text\": {}", q(&text)));
        }
        out.push_str(&fields.join(",\n"));
        out.push_str("\n    }");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin;

    fn de_meta() -> PackMeta {
        builtin::meta_of("de").unwrap()
    }

    #[test]
    fn every_builtin_table_survives_json_and_ftl_round_trips() {
        let reference = builtin::english();
        for lang in ["en", "de"] {
            let table = builtin::table_of(lang).unwrap();
            let meta = builtin::meta_of(lang).unwrap();
            let json = export_json(&reference, &table, lang, &meta, ExportOptions::default());
            let imported = parse_json(&json);
            assert!(
                imported.table.diagnostics.is_empty(),
                "{lang}: {:?}",
                imported.table.diagnostics
            );
            assert_eq!(
                imported.table.messages.len(),
                table.messages.len(),
                "{lang}"
            );
            for (k, p) in &table.messages {
                assert_eq!(imported.table.messages.get(k), Some(p), "{lang}: {k}");
            }
            // JSON -> ftl -> parse
            let mut back = Table::default();
            for (_, text) in import_to_ftl(&imported) {
                let t = fluent::parse(&text);
                assert!(t.diagnostics.is_empty(), "{lang}: {:?}", t.diagnostics);
                back.messages.extend(t.messages);
            }
            assert_eq!(back.messages, table.messages, "{lang}");
        }
    }

    #[test]
    fn the_ftl_template_parses_and_is_complete() {
        let reference = builtin::english();
        for (name, text) in export_ftl(
            &reference,
            &Table::default(),
            "fr",
            ExportOptions::default(),
        ) {
            let t = fluent::parse(&text);
            assert!(t.diagnostics.is_empty(), "{name}: {:?}", t.diagnostics);
            for k in &t.order {
                assert_eq!(t.messages[k], reference.table.messages[k], "{k}");
            }
        }
        let de = builtin::table_of("de").unwrap();
        let files = export_ftl(&reference, &de, "de", ExportOptions::default());
        let total: usize = files
            .iter()
            .map(|(_, t)| fluent::parse(t).messages.len())
            .sum();
        assert_eq!(total, reference.len());
    }

    #[test]
    fn untranslated_json_entries_are_skipped_and_partial_plurals_rejected() {
        let src = r#"{
  "format": "opts-l10n/1",
  "messages": {
    "a": { "source": "A", "text": "" },
    "b": { "variable": "n", "forms": { "one": "x", "other": "" } },
    "c": { "variable": "n", "forms": { "one": "{ $n } Stück", "other": "{ $n } Stücke" } },
    "d": 5
  }
}"#;
        let i = parse_json(src);
        assert!(!i.table.messages.contains_key("a"));
        assert!(i.table.messages.contains_key("c"));
        let lines: Vec<(usize, &str)> = i
            .table
            .diagnostics
            .iter()
            .map(|d| (d.line, d.key.as_deref().unwrap_or("")))
            .collect();
        assert_eq!(lines, vec![(5, "b"), (7, "d")], "{lines:?}");
    }

    #[test]
    fn flat_json_and_broken_json_are_handled() {
        let i = parse_json("{\"a\": \"Hallo\", \"b\": \"{ $x }\"}");
        assert_eq!(i.table.messages.len(), 2);
        let broken = parse_json("{\"a\": \n  \"x\",,");
        assert_eq!(broken.table.messages.len(), 0);
        assert_eq!(broken.table.diagnostics.len(), 1);
        assert!(broken.table.diagnostics[0].line >= 1);
        assert_eq!(parse_json("[1,2]").table.diagnostics.len(), 1);
    }

    #[test]
    fn export_lists_language_plural_categories() {
        let reference = Reference::from_sources(&[(
            "e.ftl",
            "# Count.\nn =\n    { $c ->\n        [one] { $c } item\n       *[other] { $c } items\n    }\n",
        )]);
        let json = export_json(
            &reference,
            &Table::default(),
            "ru",
            &de_meta(),
            ExportOptions::default(),
        );
        assert!(json.contains("\"few\""), "{json}");
        assert!(json.contains("\"context\": \"Count.\""), "{json}");
        let _ = parse_json(&json);
    }
}
