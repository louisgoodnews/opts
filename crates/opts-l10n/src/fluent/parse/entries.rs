//! Entries of a Fluent file: grouping lines into entries and dedenting them.

use super::*;

pub(super) type Entry = (usize, String, Vec<(usize, String)>, String);

/// Parses an `.ftl` source. Problems never stop the parse: the faulty message is skipped, the
/// rest is read, and every problem is listed in [`Table::diagnostics`] with its line.
pub fn parse(source: &str) -> Table {
    let mut bundle = Table::default();
    let scanner = LineScanner::new(&mut bundle);
    let entries = scanner.scan(source);
    for entry in entries {
        process_entry(entry, &mut bundle);
    }
    bundle.errors = bundle.diagnostics.iter().map(ToString::to_string).collect();
    bundle
}

pub(super) fn process_entry(entry: Entry, bundle: &mut Table) {
    let (line, id, body, comment) = entry;
    let (text, line_of) = dedent(&body);
    match parse_pattern(&text) {
        Ok(p) => {
            if bundle.messages.insert(id.clone(), p).is_some() {
                bundle.diagnostics.push(Diagnostic {
                    line,
                    key: Some(id.clone()),
                    message: "duplicate key (the later definition wins)".into(),
                });
            } else {
                bundle.order.push(id.clone());
            }
            bundle.lines.insert(id.clone(), line);
            if !comment.is_empty() {
                bundle.comments.insert(id, comment);
            }
        }
        Err(e) => {
            let at = e
                .offset
                .map(|o| text.chars().take(o).filter(|c| *c == '\n').count())
                .and_then(|i| line_of.get(i).copied())
                .unwrap_or(line);
            bundle.diagnostics.push(Diagnostic {
                line: at,
                key: Some(id),
                message: e.message,
            });
        }
    }
}

/// Joins the body lines: the first line keeps its text, continuation lines lose their common
/// indent. Also returns the source line of each output line.
pub(super) fn dedent(body: &[(usize, String)]) -> (String, Vec<usize>) {
    let first_line = body.first().map_or(1, |(n, _)| *n);
    let first = body.first().map_or("", |(_, s)| s.trim());
    let rest: Vec<&(usize, String)> = body
        .iter()
        .skip(1)
        .filter(|(_, l)| !l.trim().is_empty())
        .collect();
    let indent = rest
        .iter()
        .map(|(_, l)| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = first.to_string();
    let mut lines = vec![first_line];
    for (n, l) in body.iter().skip(1) {
        if l.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(l[indent.min(l.len())..].trim_end());
        lines.push(*n);
    }
    let lead = out.len() - out.trim_start().len();
    let dropped = out[..lead].matches('\n').count();
    let lines = lines.split_at(dropped.min(lines.len())).1.to_vec();
    (out.trim().to_string(), lines)
}
