//! Unit tests for `super` (`keys.rs`).

use super::*;

#[test]
fn letters_digits_and_function_keys_resolve() {
    assert_eq!(parse_key("W").map(|k| k.0), Some(KeyCode::KeyW));
    assert_eq!(parse_key("w").map(|k| k.0), Some(KeyCode::KeyW));
    assert_eq!(parse_key("5").map(|k| k.0), Some(KeyCode::Digit5));
    assert_eq!(parse_key("F1"), Some((KeyCode::F1, Key::F1)));
    assert_eq!(parse_key("f12").map(|k| k.0), Some(KeyCode::F12));
    assert_eq!(parse_key("Escape").map(|k| k.0), Some(KeyCode::Escape));
    assert_eq!(parse_key("escape").map(|k| k.0), Some(KeyCode::Escape));
    assert_eq!(parse_key("ArrowUp").map(|k| k.0), Some(KeyCode::ArrowUp));
}

#[test]
fn names_round_trip() {
    for name in [
        "L",
        "7",
        "F5",
        "NumpadDecimal",
        "PageUp",
        "Space",
        "ScrollLock",
    ] {
        let (code, _) = parse_key(name).unwrap();
        assert_eq!(key_name(code).as_deref(), Some(name));
    }
    assert_eq!(key_name(KeyCode::KeyZ).as_deref(), Some("Z"));
    assert_eq!(key_name(KeyCode::IntlBackslash), None);
}

#[test]
fn unknown_names_do_not_resolve() {
    assert!(parse_key("F13").is_none());
    assert!(parse_key("Nope").is_none());
    assert!(parse_key("").is_none());
    assert!(!key_names().is_empty());
}
