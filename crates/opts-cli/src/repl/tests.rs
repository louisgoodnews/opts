//! Unit tests for the REPL: shell-like word splitting and detection of incomplete JSON input.
use super::{is_incomplete, split_words};

#[test]
fn words_split_like_a_shell() {
    assert_eq!(
        split_words(r#"input.key W --duration 2000"#),
        ["input.key", "W", "--duration", "2000"]
    );
    assert_eq!(
        split_words(r#"report "hello  world" 'a b'"#),
        ["report", "hello  world", "a b"]
    );
    assert_eq!(split_words(r#"x "" y"#), ["x", "", "y"]);
    assert_eq!(split_words(r"a\ b"), ["a b"]);
}

#[test]
fn unbalanced_json_is_incomplete() {
    assert!(is_incomplete(r#"entity.spawn --json {"asset":"a""#));
    assert!(is_incomplete("batch [ "));
    assert!(!is_incomplete(r#"report "text with { brace""#));
    assert!(!is_incomplete(r#"x --json {"a":[1,2]}"#));
}
