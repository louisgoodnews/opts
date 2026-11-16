//! Unit tests of `binary`.

use super::*;

fn name(s: &str) -> Vec<u8> {
    let mut v = NAME.to_le_bytes().to_vec();
    v.extend((s.len() as u32).to_le_bytes());
    v.extend(s.as_bytes());
    v
}

#[test]
fn single_triangle_mesh_tokenizes() {
    let mut b = Vec::new();
    b.extend(name("Mesh"));
    b.extend(OBRACE.to_le_bytes());
    b.extend(INTEGER_LIST.to_le_bytes());
    b.extend(1u32.to_le_bytes());
    b.extend(3u32.to_le_bytes());
    b.extend(FLOAT_LIST.to_le_bytes());
    b.extend(9u32.to_le_bytes());
    for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        b.extend(f.to_le_bytes());
    }
    b.extend(CBRACE.to_le_bytes());
    let t = tokenize_binary(&b, 32).expect("tokens");
    assert_eq!(t[0].kind, TokenKind::Ident("Mesh".into()));
    assert_eq!(t[2].kind, TokenKind::Number("3".into()));
    assert_eq!(
        t.iter()
            .filter(|t| matches!(t.kind, TokenKind::Number(_)))
            .count(),
        10
    );
    assert_eq!(t.last().map(|t| &t.kind), Some(&TokenKind::CloseBrace));
}

#[test]
fn truncated_stream_is_an_error_not_a_panic() {
    let mut b = name("Mesh");
    b.extend(FLOAT_LIST.to_le_bytes());
    b.extend(1000u32.to_le_bytes());
    assert!(tokenize_binary(&b, 32).is_err());
}
