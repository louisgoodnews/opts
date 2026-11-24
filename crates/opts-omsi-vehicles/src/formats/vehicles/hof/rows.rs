//! Row helpers of the `.hof` parser: keyword test, parameter lookup and the terminus and bus-stop
//! list rows.

use super::*;

/// Checks whether `line` starts at column 0 with a `[` keyword.
pub(super) fn is_keyword(line: &str) -> bool {
    let trimmed = line.trim_end_matches(['\r', '\n', ' ', '\t']);
    trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2
}

/// Returns the next non-empty, non-comment parameter line.
pub(super) fn next_param<'a>(lines: &'a [&'a str], mut i: usize) -> Option<(&'a str, usize)> {
    while i < lines.len() {
        let raw = lines[i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if is_keyword(raw) {
            return None;
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("...") {
            return Some((trimmed, i + 1));
        }
        i += 1;
    }
    None
}

/// Returns the next parameter line preserving internal content (even if empty or spaces),
/// stopping only on keywords.
pub(super) fn next_param_raw<'a>(lines: &'a [&'a str], mut i: usize) -> Option<(&'a str, usize)> {
    while i < lines.len() {
        let raw = lines[i].trim_end_matches(['\r', '\n']);
        if is_keyword(raw) {
            return None;
        }
        let trimmed = raw.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("...") {
            i += 1;
            continue;
        }
        return Some((raw.trim_matches(['\r', '\n']), i + 1));
    }
    None
}

/// Parses one row of an `[addterminus_list]` block.
pub(super) fn parse_terminus_list_row(row: &str, stringcount: usize) -> Option<HofTerminus> {
    let mut parts: Vec<&str> = row.split('\t').collect();
    if parts.first().is_some_and(|p| p.trim().is_empty()) {
        parts.remove(0);
    }
    if parts.is_empty() {
        return None;
    }

    let mut all_exit = false;
    let mut cursor = 0;

    if parts[0].trim().eq_ignore_ascii_case("{allex}") {
        all_exit = true;
        cursor += 1;
    }

    if cursor >= parts.len() {
        return None;
    }

    let code: i32 = parts[cursor].trim().parse().ok()?;
    cursor += 1;

    let ident = if cursor < parts.len() {
        parts[cursor].trim().to_string()
    } else {
        String::new()
    };
    cursor += 1;

    let mut strings = Vec::new();
    while cursor < parts.len() && strings.len() < stringcount {
        strings.push(parts[cursor].to_string());
        cursor += 1;
    }
    while strings.len() < stringcount {
        strings.push(String::new());
    }

    Some(HofTerminus {
        code,
        ident,
        strings,
        all_exit,
    })
}

/// Parses one row of an `[addbusstop_list]` block.
pub(super) fn parse_busstop_list_row(row: &str, stringcount: usize) -> Option<HofBusStop> {
    let mut parts: Vec<&str> = row.split('\t').collect();
    if parts.first().is_some_and(|p| p.trim().is_empty()) {
        parts.remove(0);
    }
    if parts.is_empty() {
        return None;
    }

    let ident = parts[0].trim().to_string();
    let mut strings = Vec::new();
    for p in parts.iter().skip(1).take(stringcount) {
        strings.push((*p).to_string());
    }
    while strings.len() < stringcount {
        strings.push(String::new());
    }

    Some(HofBusStop { ident, strings })
}
