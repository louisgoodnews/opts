//! Tokeniser of `.osc` scripts: comments, quoted strings and whitespace-separated words.

/// One script token with the 1-based line it started on.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token text; for string literals the text between the quotes.
    pub text: String,
    /// `true` for `"quoted"` string literals.
    pub is_string: bool,
    /// Source line (1-based).
    pub line: usize,
}

/// Splits `source` into tokens. A `'` starts a comment that runs to the end of the line (also
/// after code); `"…"` is one token (strings cannot contain `'`, `osc.md`).
#[must_use]
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for (n, raw) in source.lines().enumerate() {
        let line = n + 1;
        let mut chars = raw.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c == '\'' {
                break;
            }
            if c.is_whitespace() || c == '\u{0}' {
                chars.next();
                continue;
            }
            if c == '"' {
                chars.next();
                let mut text = String::new();
                for ch in chars.by_ref() {
                    if ch == '"' {
                        break;
                    }
                    text.push(ch);
                }
                out.push(Token {
                    text,
                    is_string: true,
                    line,
                });
                continue;
            }
            let mut text = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() || ch == '\'' || ch == '"' {
                    break;
                }
                text.push(ch);
                chars.next();
            }
            out.push(Token {
                text,
                is_string: false,
                line,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(s: &str) -> Vec<String> {
        tokenize(s).into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn comments_run_to_end_of_line_and_may_trail_code() {
        assert!(tokenize("'only a comment").is_empty());
        assert_eq!(texts("1 2 + 'sum\n3"), ["1", "2", "+", "3"]);
    }

    #[test]
    fn quoted_strings_are_single_tokens() {
        let t = tokenize("\"a b\" \"\" x");
        assert_eq!(t.len(), 3);
        assert!(t[0].is_string && t[0].text == "a b");
        assert!(t[1].is_string && t[1].text.is_empty());
        assert!(!t[2].is_string);
    }

    #[test]
    fn lines_are_tracked() {
        let t = tokenize("a\n\nb c");
        assert_eq!(t.iter().map(|t| t.line).collect::<Vec<_>>(), [1, 3, 3]);
    }
}
