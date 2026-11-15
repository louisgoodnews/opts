//! KeyValues text: the tokenizer and parser of Steam's `libraryfolders.vdf` and `appmanifest` files.

/// A KeyValues value: a string or a block of key-value pairs (keys keep their order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kv {
    /// A quoted or bare string.
    Str(String),
    /// A `{ … }` block.
    Block(Vec<(String, Kv)>),
}

impl Kv {
    /// The value of `key` in a block (keys compare without case, as Steam does).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Kv> {
        match self {
            Kv::Block(pairs) => pairs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Kv::Str(_) => None,
        }
    }

    /// The string, if this is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Kv::Str(s) => Some(s),
            Kv::Block(_) => None,
        }
    }
}

/// Splits KeyValues text into tokens: strings (quoted with `\\`, `\"`, `\n`, `\t` escapes, or
/// bare), `{` and `}`. `//` comments and `[$WIN32]`-style conditions are skipped.
pub(super) fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => break,
                        },
                        _ => s.push(c),
                    }
                }
                out.push(Token::Str(s));
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '[' => {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || matches!(n, '"' | '{' | '}') {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                out.push(Token::Str(s));
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Str(String),
    Open,
    Close,
}

/// Parses KeyValues text into the top-level block. Malformed input yields what was read up to
/// the error (Steam's files are machine-written; a truncated file still gives its first entries).
#[must_use]
pub fn parse(text: &str) -> Kv {
    let tokens = tokens(text);
    let mut pos = 0;
    Kv::Block(block(&tokens, &mut pos))
}

pub(super) fn block(tokens: &[Token], pos: &mut usize) -> Vec<(String, Kv)> {
    let mut pairs = Vec::new();
    while let Some(t) = tokens.get(*pos) {
        *pos += 1;
        let key = match t {
            Token::Str(k) => k.clone(),
            Token::Close => break,
            Token::Open => continue,
        };
        match tokens.get(*pos) {
            Some(Token::Str(v)) => {
                *pos += 1;
                pairs.push((key, Kv::Str(v.clone())));
            }
            Some(Token::Open) => {
                *pos += 1;
                pairs.push((key, Kv::Block(block(tokens, pos))));
            }
            Some(Token::Close) | None => break,
        }
    }
    pairs
}
