//! Unit tests for `super` (`container.rs`).

use super::*;
use serde_json::json;

fn sample() -> SaveFile {
    let mut f = SaveFile::new(SaveMeta {
        name: "Test".into(),
        map: "Berlin".into(),
        vehicles: vec!["Bus".into()],
        play_time_s: 12.5,
        ..Default::default()
    });
    f.sections.insert(
        "x".into(),
        SectionBlob {
            version: 3,
            data: json!({"a": 1}),
        },
    );
    f
}

#[test]
fn a_file_round_trips() {
    let f = sample();
    assert_eq!(
        SaveFile::decode(&f.encode().expect("encode")).expect("decode"),
        f
    );
}

#[test]
fn a_newer_format_is_refused_even_when_the_layout_changed() {
    let text = json!({"magic": MAGIC, "format_version": 99, "something": "else"}).to_string();
    assert_eq!(
        SaveFile::decode(&text),
        Err(SaveError::NewerFormat {
            found: 99,
            supported: FORMAT_VERSION
        })
    );
}

#[test]
fn other_json_and_garbage_are_told_apart() {
    assert_eq!(SaveFile::decode("[1,2]"), Err(SaveError::NotASave));
    assert_eq!(SaveFile::decode("{\"a\":1}"), Err(SaveError::NotASave));
    assert!(matches!(
        SaveFile::decode("{\"magic\": \"OPTS-"),
        Err(SaveError::Corrupt(_))
    ));
    assert!(matches!(
        SaveFile::decode(&json!({"magic": MAGIC}).to_string()),
        Err(SaveError::Corrupt(_))
    ));
}

#[test]
fn oversized_sections_are_refused_on_write() {
    let mut f = sample();
    f.sections.insert(
        "big".into(),
        SectionBlob {
            version: 1,
            data: json!("x".repeat(MAX_SECTION_BYTES + 1)),
        },
    );
    assert!(matches!(f.encode(), Err(SaveError::TooLarge(_))));
}

#[test]
fn names_are_cleaned_and_slugged() {
    assert_eq!(clean_name("  Fahrt\u{0}\n 1 ").as_deref(), Ok("Fahrt 1"));
    assert!(clean_name("   ").is_err());
    assert_eq!(
        clean_name(&"a".repeat(100)).map(|n| n.chars().count()),
        Ok(MAX_NAME_CHARS)
    );
    assert_eq!(slug("Meine Fahrt, Nr. 1!"), "meine-fahrt-nr-1");
    assert_eq!(slug("???"), "save");
    assert!(valid_id(&slug("../../etc/passwd")));
    assert!(!valid_id("../x"));
    assert!(!valid_id(""));
}
