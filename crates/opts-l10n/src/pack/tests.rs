//! Unit tests of `pack.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

#[test]
fn language_codes() {
    for ok in ["fr", "pt-BR", "zh-Hans", "yue"] {
        assert!(valid_code(ok), "{ok}");
    }
    for bad in ["", "f", "FR", "fr_FR", "english", "../x", "fr-", "fr-x"] {
        assert!(!valid_code(bad), "{bad}");
    }
}

#[test]
fn meta_errors_name_the_field_and_line() {
    let (m, issues) = parse_meta(
        "{\n  \"code\": \"fr\",\n  \"name\": \"French\",\n  \"native_name\": \"\",\n  \"version\": \"1\",\n  \"license\": \"MIT OR Apache-2.0\",\n  \"direction\": \"up\"\n}\n",
        "fr",
    );
    assert!(m.is_none());
    let at: Vec<(Option<usize>, &str)> = issues
        .iter()
        .map(|i| (i.line, i.message.as_str()))
        .collect();
    assert!(
        at.iter()
            .any(|(l, m)| *l == Some(4) && m.contains("native_name")),
        "{at:?}"
    );
    assert!(
        at.iter()
            .any(|(l, m)| *l == Some(7) && m.contains("direction")),
        "{at:?}"
    );
    let (_, broken) = parse_meta("{ \"code\": ", "fr");
    assert_eq!(broken[0].code, "pack-json");
}

#[test]
fn code_must_equal_the_folder() {
    let text = "{\"code\":\"de\",\"name\":\"a\",\"native_name\":\"b\",\"version\":\"1\",\"license\":\"MIT OR Apache-2.0\",\"authors\":[\"x\"]}";
    assert!(parse_meta(text, "fr").0.is_none());
    assert!(parse_meta(text, "de").0.is_some());
}

#[test]
fn meta_round_trips_through_its_json() {
    let m = super::super::builtin::meta_of("de").unwrap();
    let (back, issues) = parse_meta(&m.to_json(), "de");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(back, Some(m));
}
