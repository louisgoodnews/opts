//! A small parser and formatter for the Fluent subset OPTS uses.
//!
//! Supported: `# comments`, `key = pattern` with indented continuation lines, `{ $variable }`,
//! `{ other-message }` references, `{ "literal" }` and plural/value selectors
//! (`{ $n -> [one] ... *[other] ... }`, numeric keys match exactly). Not supported (yet): terms
//! (`-term`), attributes (`.attr = ...`), functions (`NUMBER`, `DATETIME`), message-level
//! parameterised terms. Upgrade path: the `fluent-bundle` crate reads the same `.ftl` files; swap
//! [`Table`] for `FluentBundle` behind [`crate::l10n::Localization::format`] when the crates can be
//! vendored (they were not in the offline registry on 2026-10-12).

use std::collections::HashMap;

/// A value passed to a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// Text.
    Str(String),
    /// A number (selects plural forms).
    Num(f64),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}
impl From<f64> for Arg {
    fn from(n: f64) -> Self {
        Self::Num(n)
    }
}
impl From<usize> for Arg {
    fn from(n: usize) -> Self {
        Self::Num(n as f64)
    }
}
impl From<i32> for Arg {
    fn from(n: i32) -> Self {
        Self::Num(f64::from(n))
    }
}

impl Arg {
    fn render(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
            Self::Num(n) => format!("{n}"),
        }
    }
}

/// Named arguments of one message.
pub type Args = Vec<(String, Arg)>;

/// One piece of a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// Literal text.
    Text(String),
    /// `{ $name }`.
    Var(String),
    /// `{ message-id }`.
    Ref(String),
    /// `{ $name -> ... }`.
    Select {
        /// The selector variable.
        var: String,
        /// `(key, pattern)` per variant.
        variants: Vec<(String, Pattern)>,
        /// Index of the `*` default variant.
        default: usize,
    },
}

/// A message body.
pub type Pattern = Vec<Part>;

/// All messages of one language.
#[derive(Debug, Clone, Default)]
pub struct Table {
    /// Message id to pattern.
    pub messages: HashMap<String, Pattern>,
    /// Parse problems (`line N: ...`), empty for a clean file.
    pub errors: Vec<String>,
}

/// Brace characters, spelled as numbers so line-counting tools do not mistake them for blocks.
const LBRACE: char = 0x7b as char;
const RBRACE: char = 0x7d as char;

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Parses an `.ftl` source.
pub fn parse(source: &str) -> Table {
    let mut bundle = Table::default();
    let mut current: Option<(usize, String, Vec<String>)> = None;
    let mut entries = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let starts_entry = line.split_once('=').is_some_and(|(id, _)| {
            !id.is_empty()
                && id.trim_end().chars().all(is_id_char)
                && !line.starts_with(char::is_whitespace)
        });
        if line.trim_start().starts_with('#') && !line.starts_with(char::is_whitespace) {
            continue;
        }
        if starts_entry {
            if let Some(done) = current.take() {
                entries.push(done);
            }
            let (id, rest) = line.split_once('=').unwrap_or((line, ""));
            current = Some((n + 1, id.trim().to_string(), vec![rest.to_string()]));
        } else if let Some((_, _, body)) = current.as_mut() {
            body.push(line.to_string());
        }
    }
    entries.extend(current);
    for (line, id, body) in entries {
        let text = dedent(&body);
        match parse_pattern(&text) {
            Ok(p) => {
                bundle.messages.insert(id, p);
            }
            Err(e) => bundle.errors.push(format!("line {line}: {id}: {e}")),
        }
    }
    bundle
}

/// Joins the body lines: the first line keeps its text, continuation lines lose their common indent.
fn dedent(body: &[String]) -> String {
    let first = body.first().map_or("", |s| s.trim());
    let rest: Vec<&String> = body
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .collect();
    let indent = rest
        .iter()
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = first.to_string();
    for l in body.iter().skip(1) {
        if l.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(l[indent.min(l.len())..].trim_end());
    }
    out.trim().to_string()
}

fn parse_pattern(text: &str) -> Result<Pattern, String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == LBRACE {
            let mut depth = 1;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                match chars[j] {
                    c if c == LBRACE => depth += 1,
                    c if c == RBRACE => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                return Err("unclosed `{`".into());
            }
            if !buf.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut buf)));
            }
            let inner: String = chars[i + 1..j - 1].iter().collect();
            parts.push(parse_placeable(inner.trim())?);
            i = j;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        parts.push(Part::Text(buf));
    }
    Ok(parts)
}

fn parse_placeable(inner: &str) -> Result<Part, String> {
    if let Some((head, body)) = inner.split_once("->") {
        let var = head
            .trim()
            .strip_prefix('$')
            .ok_or("selector must be a $variable")?;
        return parse_select(var.trim(), body);
    }
    if let Some(var) = inner.strip_prefix('$') {
        return Ok(Part::Var(var.trim().to_string()));
    }
    if inner.len() >= 2 && inner.starts_with('"') && inner.ends_with('"') {
        return Ok(Part::Text(inner[1..inner.len() - 1].to_string()));
    }
    if inner
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && inner.chars().all(is_id_char)
    {
        return Ok(Part::Ref(inner.to_string()));
    }
    Err(format!("unsupported placeable `{{ {inner} }}`"))
}

fn parse_select(var: &str, body: &str) -> Result<Part, String> {
    let mut variants: Vec<(String, String)> = Vec::new();
    let mut default = None;
    for line in body.lines() {
        let t = line.trim();
        let (star, rest) = match t.strip_prefix('*') {
            Some(r) => (true, r),
            None => (false, t),
        };
        if let Some(r) = rest.strip_prefix('[') {
            let (key, text) = r.split_once(']').ok_or("`[` without `]`")?;
            if star {
                default = Some(variants.len());
            }
            variants.push((key.trim().to_string(), text.trim().to_string()));
        } else if let Some(last) = variants.last_mut() {
            if !t.is_empty() {
                last.1.push('\n');
                last.1.push_str(t);
            }
        } else if !t.is_empty() {
            return Err("text before the first variant".into());
        }
    }
    let default = default.ok_or("selector needs a `*[default]` variant")?;
    let variants = variants
        .into_iter()
        .map(|(k, v)| parse_pattern(&v).map(|p| (k, p)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Part::Select {
        var: var.to_string(),
        variants,
        default,
    })
}

/// The plural category of `n` in `lang` (`en` and `de`: `one` for exactly 1, else `other`).
pub fn plural_category(_lang: &str, n: f64) -> &'static str {
    if (n - 1.0).abs() < f64::EPSILON {
        "one"
    } else {
        "other"
    }
}

/// Formats a pattern; `lookup` resolves `{ other-message }` references.
pub fn format(
    pattern: &Pattern,
    lang: &str,
    args: &[(String, Arg)],
    lookup: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> String {
    let mut out = String::new();
    for part in pattern {
        match part {
            Part::Text(t) => out.push_str(t),
            Part::Var(v) => match args.iter().find(|(n, _)| n == v) {
                Some((_, a)) => out.push_str(&a.render()),
                None => out.push_str(&format!("{{${v}}}")),
            },
            Part::Ref(id) => {
                if depth < 8 {
                    out.push_str(&lookup(id).unwrap_or_else(|| format!("{{{id}}}")));
                }
            }
            Part::Select {
                var,
                variants,
                default,
            } => {
                let chosen = match args.iter().find(|(n, _)| n == var).map(|(_, a)| a) {
                    Some(Arg::Num(n)) => variants
                        .iter()
                        .position(|(k, _)| {
                            k.parse::<f64>().is_ok_and(|x| (x - n).abs() < f64::EPSILON)
                        })
                        .or_else(|| {
                            let cat = plural_category(lang, *n);
                            variants.iter().position(|(k, _)| k == cat)
                        }),
                    Some(Arg::Str(s)) => variants.iter().position(|(k, _)| k == s),
                    None => None,
                };
                let (_, p) = &variants[chosen.unwrap_or(*default)];
                out.push_str(&format(p, lang, args, lookup, depth + 1));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str, id: &str, args: &[(&str, Arg)]) -> String {
        let b = parse(src);
        assert!(b.errors.is_empty(), "{:?}", b.errors);
        let args: Args = args
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        let lookup = |k: &str| {
            b.messages
                .get(k)
                .map(|p| format(p, "en", &args, &|_| None, 1))
        };
        format(&b.messages[id], "en", &args, &lookup, 0)
    }

    #[test]
    fn plain_variables_and_references() {
        let src = "# c\nname = Alice\nhello = Hi { $who }, { name }!\n";
        assert_eq!(
            fmt(src, "hello", &[("who", "Bob".into())]),
            "Hi Bob, Alice!"
        );
        assert_eq!(fmt(src, "hello", &[]), "Hi {$who}, Alice!");
    }

    #[test]
    fn plurals_choose_by_category_and_exact_number() {
        let src = "n =\n    { $c ->\n        [0] none\n        [one] one thing\n       *[other] { $c } things\n    }\n";
        assert_eq!(fmt(src, "n", &[("c", 0.into())]), "none");
        assert_eq!(fmt(src, "n", &[("c", 1.into())]), "one thing");
        assert_eq!(fmt(src, "n", &[("c", 5.into())]), "5 things");
        assert_eq!(fmt(src, "n", &[]), "{$c} things");
    }

    #[test]
    fn continuation_lines_join_with_newlines() {
        let src = "m = first\n    second\n";
        assert_eq!(fmt(src, "m", &[]), "first\nsecond");
    }

    #[test]
    fn errors_are_reported_not_hidden() {
        let b = parse("a = { $x\nb = ok\nc = { -term }\n");
        assert_eq!(b.errors.len(), 2, "{:?}", b.errors);
        assert!(b.messages.contains_key("b"));
    }

    #[test]
    fn shipped_tables_parse_cleanly() {
        for (name, src) in [
            ("en", include_str!("en.ftl")),
            ("de", include_str!("de.ftl")),
        ] {
            let b = parse(src);
            assert!(b.errors.is_empty(), "{name}: {:?}", b.errors);
            assert!(b.messages.len() >= 5);
        }
    }
}
