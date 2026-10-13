//! The pseudo-locale `en-XA`: English with accented letters and 40 % more length.
//!
//! It exists for tests and manual checks: text that survives `en-XA` has room to grow (German is
//! typically 30 % longer), is not clipped by fixed widths and is not built from concatenation.

use super::fluent::{Part, Pattern, Table};

/// Language tag of the pseudo-locale.
pub const PSEUDO_TAG: &str = "en-XA";

fn accent(c: char) -> char {
    match c {
        'a' => 'å',
        'b' => 'ƀ',
        'c' => 'ç',
        'd' => 'ð',
        'e' => 'é',
        'g' => 'ĝ',
        'h' => 'ĥ',
        'i' => 'í',
        'j' => 'ĵ',
        'k' => 'ķ',
        'l' => 'ļ',
        'n' => 'ñ',
        'o' => 'ö',
        'p' => 'þ',
        's' => 'š',
        't' => 'ţ',
        'u' => 'ü',
        'w' => 'ŵ',
        'y' => 'ý',
        'z' => 'ž',
        'A' => 'Å',
        'E' => 'É',
        'I' => 'Í',
        'O' => 'Ö',
        'U' => 'Ü',
        other => other,
    }
}

fn visible_len(p: &Pattern) -> usize {
    p.iter()
        .map(|part| match part {
            Part::Text(t) => t.chars().count(),
            Part::Var(_) | Part::Ref(_) => 0,
            Part::Select {
                variants, default, ..
            } => visible_len(&variants[*default].1),
        })
        .sum()
}

fn transform(p: &Pattern) -> Pattern {
    p.iter()
        .map(|part| match part {
            Part::Text(t) => Part::Text(t.chars().map(accent).collect()),
            Part::Select {
                var,
                variants,
                default,
            } => Part::Select {
                var: var.clone(),
                variants: variants
                    .iter()
                    .map(|(k, v)| (k.clone(), pad(transform(v))))
                    .collect(),
                default: *default,
            },
            other => other.clone(),
        })
        .collect()
}

/// Appends `~` so the visible text is at least 40 % longer, and brackets it.
fn pad(mut p: Pattern) -> Pattern {
    let base = visible_len(&p);
    let extra = (base * 2).div_ceil(5).max(1);
    p.push(Part::Text("~".repeat(extra)));
    p
}

/// The pseudo-localised pattern of an English one.
pub fn pseudo_pattern(p: &Pattern) -> Pattern {
    let mut out = transform(p);
    // Selects pad their variants; plain text pads once at the end.
    if !out.iter().any(|x| matches!(x, Part::Select { .. })) {
        out = pad(out);
    }
    let mut bracketed = vec![Part::Text("[".into())];
    bracketed.extend(out);
    bracketed.push(Part::Text("]".into()));
    bracketed
}

/// The whole pseudo bundle derived from the English one.
pub fn pseudo_table(en: &Table) -> Table {
    Table {
        messages: en
            .messages
            .iter()
            .map(|(k, v)| (k.clone(), pseudo_pattern(v)))
            .collect(),
        order: en.order.clone(),
        ..Table::default()
    }
}
