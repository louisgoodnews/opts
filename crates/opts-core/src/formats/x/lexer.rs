//! Comment- and separator-tolerant tokenizer for Microsoft DirectX `.x` text files.
//!
//! Handles `#` and `//` comments, literal strings with raw backslashes,
//! identifiers, floating-point/integer numbers, and curly braces.

use crate::formats::x::error::XParseError;

/// The specific classification of a token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// An alphanumeric or symbol identifier (e.g. `Mesh`, `Frame`, `Root`).
    Ident(String),
    /// A quoted string literal (e.g. `"texture.bmp"`).
    String(String),
    /// A numeric integer or floating-point literal (e.g. `4`, `-1.5`, `0.000000`).
    Number(String),
    /// An opening curly brace `{`.
    OpenBrace,
    /// A closing curly brace `}`.
    CloseBrace,
    /// A delimiter separator (comma `,` or semicolon `;`).
    Separator(char),
}

/// A lexical token tagged with its starting byte offset in the source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token payload.
    pub kind: TokenKind,
    /// Byte offset where this token began.
    pub offset: usize,
}

impl Token {
    /// Creates a new token.
    #[inline]
    pub fn new(kind: TokenKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// Returns the identifier string if this token is an identifier.
    pub fn as_ident(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Attempts to parse the token as an `f32`.
    pub fn to_f32(&self) -> Result<f32, XParseError> {
        match &self.kind {
            TokenKind::Number(s) => s.parse::<f32>().map_err(|_| XParseError::InvalidFloat {
                offset: self.offset,
                value: s.clone(),
            }),
            _ => Err(XParseError::UnexpectedToken {
                offset: self.offset,
                expected: "float number".to_string(),
                found: format!("{:?}", self.kind),
            }),
        }
    }

    /// Attempts to parse the token as a `u32`.
    pub fn to_u32(&self) -> Result<u32, XParseError> {
        match &self.kind {
            TokenKind::Number(s) => s.parse::<u32>().map_err(|_| XParseError::InvalidInteger {
                offset: self.offset,
                value: s.clone(),
            }),
            _ => Err(XParseError::UnexpectedToken {
                offset: self.offset,
                expected: "unsigned integer".to_string(),
                found: format!("{:?}", self.kind),
            }),
        }
    }
}

/// Tokenizes a DirectX `.x` text buffer into a sequence of [`Token`]s.
///
/// # Arguments
/// * `input` - Text byte slice following the 16-byte header.
/// * `base_offset` - Offset of the start of `input` relative to file start (normally 16).
///
/// # Errors
/// Returns [`XParseError`] if an unclosed string or unparseable construct is found.
pub fn tokenize(input: &[u8], base_offset: usize) -> Result<Vec<Token>, XParseError> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let len = input.len();

    while i < len {
        let b = input[i];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Single-line comment: // or #
        if b == b'#' || (b == b'/' && i + 1 < len && input[i + 1] == b'/') {
            while i < len && input[i] != b'\n' {
                i += 1;
            }
            if i < len && input[i] == b'\n' {
                i += 1;
            }
            continue;
        }

        let token_offset = base_offset + i;

        if b == b'{' {
            tokens.push(Token::new(TokenKind::OpenBrace, token_offset));
            i += 1;
            continue;
        }

        if b == b'}' {
            tokens.push(Token::new(TokenKind::CloseBrace, token_offset));
            i += 1;
            continue;
        }

        if b == b',' || b == b';' {
            tokens.push(Token::new(TokenKind::Separator(b as char), token_offset));
            i += 1;
            continue;
        }

        // Quoted string literal
        if b == b'"' {
            i += 1; // skip opening quote
            let str_start = i;
            let mut s = String::new();
            while i < len && input[i] != b'"' {
                if input[i] == b'\\' && i + 1 < len && input[i + 1] == b'"' {
                    s.push('"');
                    i += 2;
                } else {
                    s.push(input[i] as char);
                    i += 1;
                }
            }
            if i >= len {
                return Err(XParseError::UnexpectedEof(base_offset + str_start));
            }
            i += 1; // skip closing quote
            tokens.push(Token::new(TokenKind::String(s), token_offset));
            continue;
        }

        // Identifiers or numeric literals
        let word_start = i;
        while i < len
            && !input[i].is_ascii_whitespace()
            && input[i] != b'{'
            && input[i] != b'}'
            && input[i] != b','
            && input[i] != b';'
            && input[i] != b'#'
            && !(input[i] == b'/' && i + 1 < len && input[i + 1] == b'/')
        {
            i += 1;
        }

        let word_bytes = &input[word_start..i];
        let word = String::from_utf8_lossy(word_bytes).to_string();

        // Classify as number if float parse succeeds, otherwise ident
        if word.parse::<f64>().is_ok() {
            tokens.push(Token::new(TokenKind::Number(word), token_offset));
        } else {
            tokens.push(Token::new(TokenKind::Ident(word), token_offset));
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
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
}
