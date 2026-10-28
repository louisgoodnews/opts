//! Formatting, variable inspection, and `.ftl` source serialisation for parsed Fluent patterns.

use super::*;

/// The variables (`$name`, selectors included) and message references (plain ids) a pattern
/// uses, sorted.
#[must_use]
pub fn variables(pattern: &Pattern) -> std::collections::BTreeSet<String> {
    fn walk(p: &Pattern, out: &mut std::collections::BTreeSet<String>) {
        for part in p {
            match part {
                Part::Text(_) => {}
                Part::Var(v) => {
                    out.insert(format!("${v}"));
                }
                Part::Ref(r) => {
                    out.insert(r.clone());
                }
                Part::Select { var, variants, .. } => {
                    out.insert(format!("${var}"));
                    for (_, v) in variants {
                        walk(v, out);
                    }
                }
            }
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(pattern, &mut out);
    out
}

/// The pattern as `.ftl` value text. Parsing the result gives the same pattern back; a select
/// takes several lines (indent them under the key when writing a file, see [`to_ftl_entry`]).
#[must_use]
pub fn to_source(pattern: &Pattern) -> String {
    let mut out = String::new();
    for part in pattern {
        match part {
            Part::Text(t) => {
                let plain = !t.contains([LBRACE, RBRACE]);
                if plain {
                    out.push_str(t);
                } else {
                    for c in t.chars() {
                        if c == LBRACE || c == RBRACE {
                            out.push(LBRACE);
                            out.push_str(" \"");
                            out.push(c);
                            out.push_str("\" ");
                            out.push(RBRACE);
                        } else {
                            out.push(c);
                        }
                    }
                }
            }
            Part::Var(v) => out.push_str(&format!("{LBRACE} ${v} {RBRACE}")),
            Part::Ref(r) => out.push_str(&format!("{LBRACE} {r} {RBRACE}")),
            Part::Select {
                var,
                variants,
                default,
            } => {
                out.push_str(&format!("{LBRACE} ${var} ->\n"));
                for (i, (k, v)) in variants.iter().enumerate() {
                    let star = if i == *default { '*' } else { ' ' };
                    out.push_str(&format!("  {star}[{k}] {}\n", to_source(v)));
                }
                out.push(RBRACE);
            }
        }
    }
    out
}

/// One `key = value` entry as `.ftl` text, with `comment` lines above and continuation lines
/// indented. Ends with a newline.
#[must_use]
pub fn to_ftl_entry(key: &str, pattern: &Pattern, comment: &str) -> String {
    let mut out = String::new();
    for line in comment.lines() {
        out.push_str(&format!("# {line}\n"));
    }
    let src = to_source(pattern);
    if src.contains('\n') {
        out.push_str(&format!("{key} =\n"));
        for line in src.lines() {
            out.push_str(&format!("    {line}\n"));
        }
    } else {
        out.push_str(&format!("{key} = {src}\n"));
    }
    out
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
