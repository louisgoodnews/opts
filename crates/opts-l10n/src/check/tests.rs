//! Unit tests of `check.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn reference() -> Reference {
    Reference::from_sources(&[(
        "en.ftl",
        "# Say hi.\nhello = Hi { $who }\nitems =\n    { $count ->\n        [one] { $count } item\n       *[other] { $count } items\n    }\nplain = Save the file now\n",
    )])
}

fn check(src: &str, lang: &str) -> Checked {
    validate(&fluent::parse(src), "t.ftl", lang, &reference())
}

fn codes(c: &Checked) -> Vec<&'static str> {
    c.issues.iter().map(|i| i.code).collect()
}

#[test]
fn a_good_translation_passes_clean() {
    let c = check(
        "hello = Salut { $who }\nplain = Enregistre le fichier maintenant\n",
        "fr",
    );
    assert!(c.issues.is_empty(), "{:?}", c.issues);
    assert_eq!(c.table.messages.len(), 2);
}

#[test]
fn unknown_empty_and_mismatched_messages_are_skipped_with_their_line() {
    let c = check("hello = Salut\nnope = x\nplain =\nitems = x\n", "fr");
    let found: Vec<(&str, Option<usize>)> = c.issues.iter().map(|i| (i.code, i.line)).collect();
    assert!(found.contains(&("variable-mismatch", Some(1))), "{found:?}");
    assert!(found.contains(&("unknown-key", Some(2))), "{found:?}");
    assert!(found.contains(&("empty", Some(3))), "{found:?}");
    assert!(c.table.messages.is_empty(), "{:?}", c.table.messages.keys());
}

#[test]
fn copies_and_lengths_and_plural_forms_are_warnings() {
    let c = check("plain = Save the file now\nhello = Salut { $who }\nitems =\n    { $count ->\n       *[other] { $count } x\n    }\n", "ru");
    assert!(codes(&c).contains(&"untranslated-copy"), "{:?}", c.issues);
    assert!(codes(&c).contains(&"plural-forms"), "{:?}", c.issues);
    assert_eq!(c.table.messages.len(), 3, "warnings keep the message");
}

#[test]
fn stats_count_missing_and_copies() {
    let r = reference();
    let c = check("plain = Save the file now\nhello = Salut { $who }\n", "fr");
    let s = stats(&r, &c.table, "fr");
    assert_eq!((s.total, s.present, s.translated, s.copied), (3, 2, 1, 1));
    assert_eq!(s.missing, vec!["items".to_string()]);
}

#[test]
fn issue_display_is_editor_clickable() {
    let c = check("hello = Salut\n", "fr");
    let text = c.issues[0].to_string();
    assert!(
        text.starts_with("t.ftl:1: error[variable-mismatch]:"),
        "{text}"
    );
}

#[test]
fn revision_changes_with_the_source() {
    let a = reference().revision();
    let b = Reference::from_sources(&[("e.ftl", "hello = Hi\n")]).revision();
    assert_ne!(a, b);
    assert_eq!(a.len(), 8);
}
