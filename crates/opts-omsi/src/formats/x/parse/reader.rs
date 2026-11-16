//! Token cursor of the DirectX `.x` parser: peeks, consumes and checks braces, numbers and
//! separators, and skips unknown blocks.

use crate::formats::x::error::XParseError;
use crate::formats::x::lexer::{Token, TokenKind};

pub(super) struct TokenReader<'a> {
    tokens: &'a [Token],
    pub(super) pos: usize,
}

impl<'a> TokenReader<'a> {
    pub(super) fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    pub(super) fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    pub(super) fn next(&mut self) -> Result<&'a Token, XParseError> {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            Ok(tok)
        } else {
            let last_offset = self.tokens.last().map(|t| t.offset).unwrap_or(0);
            Err(XParseError::UnexpectedEof(last_offset))
        }
    }

    pub(super) fn skip_separators(&mut self) {
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, TokenKind::Separator(_)) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    pub(super) fn expect_open_brace(&mut self) -> Result<(), XParseError> {
        self.skip_separators();
        // Data objects may carry an optional instance name (`MeshTextureCoords c1 {`,
        // `FrameTransformMatrix relative {`); it is not needed for geometry.
        while matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Ident(_)))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::OpenBrace)
            )
        {
            self.pos += 1;
        }
        let tok = self.next()?;
        if tok.kind == TokenKind::OpenBrace {
            Ok(())
        } else {
            Err(XParseError::UnexpectedToken {
                offset: tok.offset,
                expected: "'{'".to_string(),
                found: format!("{:?}", tok.kind),
            })
        }
    }

    pub(super) fn expect_close_brace(&mut self) -> Result<(), XParseError> {
        self.skip_separators();
        // A file that ends inside open blocks (missing final braces) is accepted: everything
        // read so far is valid geometry.
        if self.peek().is_none() {
            return Ok(());
        }
        let tok = self.next()?;
        if tok.kind == TokenKind::CloseBrace {
            Ok(())
        } else {
            Err(XParseError::UnexpectedToken {
                offset: tok.offset,
                expected: "'}'".to_string(),
                found: format!("{:?}", tok.kind),
            })
        }
    }

    pub(super) fn parse_f32(&mut self) -> Result<f32, XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        tok.to_f32()
    }

    pub(super) fn parse_u32(&mut self) -> Result<u32, XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        tok.to_u32()
    }

    pub(super) fn skip_unknown_block(&mut self) -> Result<(), XParseError> {
        // Find opening brace first
        while let Some(tok) = self.peek() {
            if tok.kind == TokenKind::OpenBrace {
                break;
            }
            self.pos += 1;
        }
        self.expect_open_brace()?;
        let mut depth = 1;
        while depth > 0 {
            if self.peek().is_none() {
                break;
            }
            let tok = self.next()?;
            match tok.kind {
                TokenKind::OpenBrace => depth += 1,
                TokenKind::CloseBrace => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }
}
