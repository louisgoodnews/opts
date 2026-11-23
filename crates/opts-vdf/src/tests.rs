//! Parser tests: nested blocks, case-insensitive keys, escapes and truncated input.
use super::{parse, Kv};

#[test]
fn nested_blocks_and_strings_are_read() {
    let kv = parse("\"a\" { \"b\" \"x\" \"c\" { \"d\" \"y\" } }");
    assert_eq!(
        kv.get("a").and_then(|a| a.get("b")).and_then(Kv::as_str),
        Some("x")
    );
    let inner = kv
        .get("a")
        .and_then(|a| a.get("C"))
        .and_then(|c| c.get("d"));
    assert_eq!(inner.and_then(Kv::as_str), Some("y"));
}

#[test]
fn backslash_and_quote_escapes_are_read() {
    let kv = parse(r#""1" "D:\\SteamLibrary" "2" "E:\\Games\\Steam \"Fast\"""#);
    assert_eq!(kv.get("1").and_then(Kv::as_str), Some(r"D:\SteamLibrary"));
    assert_eq!(
        kv.get("2").and_then(Kv::as_str),
        Some(r#"E:\Games\Steam "Fast""#)
    );
}

#[test]
fn comments_and_empty_input_give_an_empty_block() {
    assert_eq!(parse(""), Kv::Block(Vec::new()));
    assert_eq!(parse("// only a comment\n"), Kv::Block(Vec::new()));
}

#[test]
fn truncated_text_gives_what_was_read() {
    let kv = parse("\"a\" { \"b\" \"x\" \"c\" { \"d\"");
    assert_eq!(
        kv.get("a").and_then(|a| a.get("b")).and_then(Kv::as_str),
        Some("x")
    );
}
