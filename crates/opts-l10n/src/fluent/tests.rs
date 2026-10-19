//! Unit tests of `fluent.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::plural::plural_category;
use super::*;

fn fmt(src: &str, id: &str, args: &[(&str, Arg)]) -> String {
    let b = parse(src);
    assert!(b.errors.is_empty(), "{:?}", b.errors);
    let args: Args = args
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let lookup = |k: &str| {
        b.messages
            .get(k)
            .map(|p| format(p, "en", &args, &|_| None, 1))
    };
    format(&b.messages[id], "en", &args, &lookup, 0)
}

#[test]
fn plain_variables_and_references() {
    let src = "# c\nname = Alice\nhello = Hi { $who }, { name }!\n";
    assert_eq!(
        fmt(src, "hello", &[("who", "Bob".into())]),
        "Hi Bob, Alice!"
    );
    assert_eq!(fmt(src, "hello", &[]), "Hi {$who}, Alice!");
}

#[test]
fn plurals_choose_by_category_and_exact_number() {
    let src = "n =\n    { $c ->\n        [0] none\n        [one] one thing\n       *[other] { $c } things\n    }\n";
    assert_eq!(fmt(src, "n", &[("c", 0.into())]), "none");
    assert_eq!(fmt(src, "n", &[("c", 1.into())]), "one thing");
    assert_eq!(fmt(src, "n", &[("c", 5.into())]), "5 things");
    assert_eq!(fmt(src, "n", &[]), "{$c} things");
}

#[test]
fn continuation_lines_join_with_newlines() {
    let src = "m = first\n    second\n";
    assert_eq!(fmt(src, "m", &[]), "first\nsecond");
}

#[test]
fn errors_are_reported_not_hidden() {
    let b = parse("a = { $x\nb = ok\nc = { -term }\n");
    assert_eq!(b.errors.len(), 2, "{:?}", b.errors);
    assert!(b.messages.contains_key("b"));
}

#[test]
fn problems_carry_their_line_and_key() {
    let b = parse("# c\nok = fine\nbad = { $x\nstray text\nok = twice\n");
    let lines: Vec<(usize, Option<&str>)> = b
        .diagnostics
        .iter()
        .map(|d| (d.line, d.key.as_deref()))
        .collect();
    assert!(lines.contains(&(3, Some("bad"))), "{lines:?}");
    assert!(lines.contains(&(4, None)), "{lines:?}");
    assert!(lines.contains(&(5, Some("ok"))), "{lines:?}");
    assert_eq!(b.messages["ok"].len(), 1);
}

#[test]
fn plural_rules_of_other_languages() {
    assert_eq!(plural_category("ru", 21.0), "one");
    assert_eq!(plural_category("ru", 3.0), "few");
    assert_eq!(plural_category("ru", 11.0), "many");
    assert_eq!(plural_category("pl", 5.0), "many");
    assert_eq!(plural_category("fr", 0.0), "one");
    assert_eq!(plural_category("fr", 2.0), "other");
    assert_eq!(plural_category("ja", 1.0), "other");
    assert_eq!(plural_category("de", 1.0), "one");
    assert_eq!(plural_category("ar", 0.0), "zero");
    assert_eq!(plural_category("pt-BR", 1.0), "one");
}
