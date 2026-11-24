//! Tokenizer for the binary (`bin `) DirectX `.x` variant.
//!
//! The binary stream is a sequence of 16-bit token codes (public DirectX file-format
//! documentation): names, strings, integer/float lists, punctuation and built-in type keywords.
//! It is converted to the same [`Token`] stream the text lexer produces, so the structural parser
//! does not care which encoding a file uses. List items become individual numbers; the implicit
//! separators of the binary encoding are not materialised (the parser skips them anyway).

use crate::formats::x::error::XParseError;
use crate::formats::x::lexer::{Token, TokenKind};

const NAME: u16 = 1;
const STRING: u16 = 2;
const INTEGER: u16 = 3;
const GUID: u16 = 5;
const INTEGER_LIST: u16 = 6;
const FLOAT_LIST: u16 = 7;
const OBRACE: u16 = 10;
const CBRACE: u16 = 11;
const COMMA: u16 = 19;
const SEMICOLON: u16 = 20;
const TEMPLATE: u16 = 31;
const ARRAY: u16 = 52;

/// Upper bound for a single list; guards against corrupt counts allocating gigabytes.
const MAX_LIST: usize = 50_000_000;

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], XParseError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.bytes.len());
        let Some(end) = end else {
            return Err(XParseError::UnexpectedEof(self.pos));
        };
        let s = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16, XParseError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, XParseError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

fn keyword(code: u16) -> Option<&'static str> {
    Some(match code {
        40 => "WORD",
        41 => "DWORD",
        42 => "FLOAT",
        43 => "DOUBLE",
        44 => "CHAR",
        45 => "UCHAR",
        46 => "SWORD",
        47 => "SDWORD",
        48 => "VOID",
        49 => "STRING",
        50 => "UNICODE",
        51 => "CSTRING",
        ARRAY => "array",
        TEMPLATE => "template",
        _ => return None,
    })
}

/// Converts a binary `.x` payload (everything after the 16-byte header) into tokens.
///
/// # Errors
/// Returns [`XParseError::UnexpectedEof`] for truncated data and
/// [`XParseError::UnexpectedToken`] for unknown token codes or implausible list sizes.
pub fn tokenize_binary(bytes: &[u8], float_bits: u8) -> Result<Vec<Token>, XParseError> {
    let mut r = Reader { bytes, pos: 0 };
    let mut out = Vec::new();
    while r.pos < bytes.len() {
        let at = r.pos;
        let code = r.u16()?;
        let kind = match code {
            NAME => {
                let len = r.u32()? as usize;
                TokenKind::Ident(String::from_utf8_lossy(r.take(len)?).into_owned())
            }
            STRING => {
                let len = r.u32()? as usize;
                let s = String::from_utf8_lossy(r.take(len)?).into_owned();
                r.u16()?; // terminator: comma or semicolon
                TokenKind::String(s)
            }
            INTEGER => TokenKind::Number(r.u32()?.to_string()),
            GUID => {
                r.take(16)?;
                continue;
            }
            INTEGER_LIST | FLOAT_LIST => {
                let count = r.u32()? as usize;
                if count > MAX_LIST {
                    return Err(XParseError::UnexpectedToken {
                        offset: at,
                        expected: "plausible list length".into(),
                        found: count.to_string(),
                    });
                }
                for _ in 0..count {
                    let text = if code == INTEGER_LIST {
                        r.u32()?.to_string()
                    } else if float_bits == 64 {
                        let b = r.take(8)?;
                        let mut a = [0u8; 8];
                        a.copy_from_slice(b);
                        (f64::from_le_bytes(a) as f32).to_string()
                    } else {
                        let b = r.take(4)?;
                        f32::from_le_bytes([b[0], b[1], b[2], b[3]]).to_string()
                    };
                    out.push(Token::new(TokenKind::Number(text), at));
                }
                continue;
            }
            OBRACE => TokenKind::OpenBrace,
            CBRACE => TokenKind::CloseBrace,
            COMMA => TokenKind::Separator(','),
            SEMICOLON => TokenKind::Separator(';'),
            12..=18 => TokenKind::Separator(' '), // ( ) [ ] < > .
            c => match keyword(c) {
                Some(k) => TokenKind::Ident(k.to_owned()),
                None => {
                    return Err(XParseError::UnexpectedToken {
                        offset: at,
                        expected: "binary token code".into(),
                        found: format!("0x{c:04X}"),
                    })
                }
            },
        };
        out.push(Token::new(kind, at));
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
