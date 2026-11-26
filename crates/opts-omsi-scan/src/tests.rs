//! Tests of the text decoding, keyword scanning and value parsing.

use super::*;

#[test]
fn blank_parameter_line_is_an_empty_value_not_the_next_text() {
    let text = "[mesh]\na.o3d\n\n[matl_transmap]\n\n\n########\n\nRauchsystem\n\n[smoke]\n-1.16\n";
    let arity = |k: &str| match k {
        "matl_transmap" => Some(1),
        "isshadow" => Some(0),
        _ => None,
    };
    let blocks = scan_blocks_with(text, arity);
    let t = blocks
        .iter()
        .find(|b| b.keyword == "matl_transmap")
        .unwrap();
    assert_eq!(
        t.parameters,
        vec![String::new()],
        "empty value, not 'Rauchsystem'"
    );
    // The old reading took the prose as the value.
    let old = scan_blocks(text);
    assert_eq!(old[1].parameters, vec!["Rauchsystem".to_string()]);
    // Following keywords are untouched.
    assert_eq!(blocks.last().unwrap().parameters, vec!["-1.16".to_string()]);
}

#[test]
fn zero_parameter_keywords_take_no_following_text() {
    let text = "[isshadow]\nnote about the mesh\n[mesh]\nb.o3d\n";
    let blocks = scan_blocks_with(text, |k| (k == "isshadow").then_some(0));
    assert!(blocks[0].parameters.is_empty());
    assert_eq!(blocks[1].parameters, vec!["b.o3d".to_string()]);
}

#[test]
fn a_texture_name_after_transmap_is_still_read() {
    let blocks = scan_blocks_with("[matl_transmap]\nmask.bmp\nstray\n", |k| {
        (k == "matl_transmap").then_some(1)
    });
    assert_eq!(blocks[0].parameters, vec!["mask.bmp".to_string()]);
}

#[test]
fn test_decode_utf8_and_cp1252() {
    let utf8_bytes = "Hallo Welt!".as_bytes();
    assert_eq!(decode_bytes(utf8_bytes), "Hallo Welt!");

    // 0xE4 is ä in CP1252 / ISO-8859-1
    let cp1252_bytes = vec![0x47, 0x65, 0x73, 0x63, 0x68, 0x66, 0x74, 0x65, 0x20, 0xE4];
    let decoded = decode_bytes(&cp1252_bytes);
    assert!(decoded.contains('ä'));

    // 0x80 is € in Windows-1252
    let euro_bytes = vec![0x80];
    assert_eq!(decode_bytes(&euro_bytes), "€");

    // UTF-16LE with BOM
    let utf16le = vec![
        0xFF, 0xFE, b'H', 0x00, b'a', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00,
    ];
    assert_eq!(decode_bytes(&utf16le), "Hallo");
}

#[test]
fn test_parse_keyword() {
    assert_eq!(parse_keyword("[currency]"), Some("currency"));
    assert_eq!(parse_keyword("[coin]\r\n"), Some("coin"));
    assert_eq!(parse_keyword("not_a_kw"), None);
    assert_eq!(parse_keyword(" [indented]"), None);
}

/// An indented `[keyword]` is not a keyword: OMSI's own editor templates carry indented help
/// blocks (`\t[NightMapMode]` followed by prose, in 5,250 `.sco` files, then the real
/// `[NightMapMode]` at the start of a line), and 505 indented `[couple_back]` lines name a
/// file that exists in only 5 % of the cases (99.6 % for those at the start of a line):
/// disabled blocks. See `docs/src/formats/text/keyword-indentation.md`.
#[test]
fn an_indented_keyword_stays_a_parameter_line() {
    let text = "[mass]\n10\n\t[NightMapMode]\n        help text\n[NightMapMode]\n2\n";
    let b = scan_blocks(text);
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].keyword, "mass");
    assert_eq!(b[0].parameters, ["10", "[NightMapMode]", "help text"]);
    assert_eq!(b[1].keyword, "nightmapmode");
    assert_eq!(b[1].parameters, ["2"]);
}

#[test]
fn test_scan_blocks() {
    let text = r#"
// Top comment banner
[currency]
Euro
2

# Comment between blocks
[coin]
coin_1.o3d
1.00

[coin]
coin_2.o3d
2.00
"#;
    let blocks = scan_blocks(text);
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].keyword, "currency");
    assert_eq!(blocks[0].parameters, vec!["Euro", "2"]);
    assert_eq!(blocks[1].keyword, "coin");
    assert_eq!(blocks[1].parameters, vec!["coin_1.o3d", "1.00"]);
    assert_eq!(blocks[2].keyword, "coin");
    assert_eq!(blocks[2].parameters, vec!["coin_2.o3d", "2.00"]);
}

#[test]
fn test_parse_float_comma() {
    assert_eq!(parse_float("1.5").unwrap(), 1.5);
    assert_eq!(parse_float("2,75").unwrap(), 2.75);
    assert_eq!(parse_f64("0,10").unwrap(), 0.10);
}
