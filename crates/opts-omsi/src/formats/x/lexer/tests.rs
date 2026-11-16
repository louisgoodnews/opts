//! Unit tests of `lexer`.

use super::*;

#[test]
fn test_tokenize_basic_block() {
    let text = b"Frame Root {\n  FrameTransformMatrix {\n    1.0, 0.0;\n  }\n}";
    let tokens = tokenize(text, 16).expect("tokenization should succeed");
    assert_eq!(tokens[0].kind, TokenKind::Ident("Frame".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::Ident("Root".to_string()));
    assert_eq!(tokens[2].kind, TokenKind::OpenBrace);
    assert_eq!(
        tokens[3].kind,
        TokenKind::Ident("FrameTransformMatrix".to_string())
    );
    assert_eq!(tokens[4].kind, TokenKind::OpenBrace);
    assert_eq!(tokens[5].kind, TokenKind::Number("1.0".to_string()));
    assert_eq!(tokens[6].kind, TokenKind::Separator(','));
    assert_eq!(tokens[7].kind, TokenKind::Number("0.0".to_string()));
    assert_eq!(tokens[8].kind, TokenKind::Separator(';'));
    assert_eq!(tokens[9].kind, TokenKind::CloseBrace);
    assert_eq!(tokens[10].kind, TokenKind::CloseBrace);
}

#[test]
fn test_tokenize_comments_and_strings() {
    let text =
        b"// Header comment\n# Hash comment\nTextureFilename { \"textures\\test.bmp\"; }";
    let tokens = tokenize(text, 0).expect("tokenization should succeed");
    assert_eq!(
        tokens[0].kind,
        TokenKind::Ident("TextureFilename".to_string())
    );
    assert_eq!(tokens[1].kind, TokenKind::OpenBrace);
    assert_eq!(
        tokens[2].kind,
        TokenKind::String("textures\\test.bmp".to_string())
    );
    assert_eq!(tokens[3].kind, TokenKind::Separator(';'));
    assert_eq!(tokens[4].kind, TokenKind::CloseBrace);
}
