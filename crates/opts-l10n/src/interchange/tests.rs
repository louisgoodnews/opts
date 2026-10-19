//! Unit tests of `interchange.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

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
