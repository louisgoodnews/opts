//! Overlay building blocks: the `[keyword]` block scan, the added-element buffer and the rule and
//! relabel readers.

use super::*;

/// Blocks an overlay adds (base layouts).
pub(super) const ADDED: [&str; 10] = [
    "object",
    "spline",
    "spline_h",
    "attachobj",
    "splineattachement",
    "splineattachement_repeater",
    "varparent",
    "spline_terrain_align",
    "spline_terrain_align_2",
    "rule",
];

pub(super) fn keyword(line: &str) -> Option<String> {
    let t = line.trim_end();
    (t.len() >= 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_ascii_lowercase())
}

pub(super) fn scan_overlay_blocks<'a>(lines: &'a [&'a str]) -> Vec<(String, usize, &'a [&'a str])> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(kw) = keyword(lines[i]) {
            let start = i + 1;
            let mut j = start;
            while j < lines.len() && keyword(lines[j]).is_none() {
                j += 1;
            }
            blocks.push((kw, i + 1, &lines[start..j]));
            i = j;
        } else {
            i += 1;
        }
    }
    blocks
}

pub(super) fn flush_added_elements(added: &mut String, out: &mut TileOverlay) {
    if added.is_empty() {
        return;
    }
    let version = out.version.unwrap_or(14);
    match MapTile::from_text(&format!("[version]\n{version}\n\n{added}")) {
        Ok(t) => out.ops.push(OverlayOp::Add(t)),
        Err(e) => out.warnings.push(format!("added records skipped: {e}")),
    }
    added.clear();
}

pub(super) fn parse_rule_op(kw: &str, line: usize, values: &[&str], out: &mut TileOverlay) {
    let path_index = values.first().and_then(|v| v.parse::<usize>().ok());
    let rule_type = values.get(1).map(|v| (*v).to_string());
    match (path_index, rule_type) {
        (Some(path_index), Some(rule_type)) if kw == "kill_rule" => {
            out.ops.push(OverlayOp::KillRule {
                path_index,
                rule_type,
            });
        }
        (Some(path_index), Some(rule_type)) => {
            let value = values.get(2).and_then(|v| parse_float(v).ok());
            let flag = values
                .get(3)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            match value {
                Some(value) => out.ops.push(OverlayOp::Rule(RuleEdit {
                    path_index,
                    rule_type,
                    value,
                    flag,
                })),
                None => out
                    .warnings
                    .push(format!("line {line}: [rule] without a value")),
            }
        }
        _ => out.warnings.push(format!("line {line}: [{kw}] damaged")),
    }
}

pub(super) fn parse_relabel_op(line: usize, body: &[&str], out: &mut TileOverlay) {
    let mut rest = body.iter().skip_while(|l| l.trim().is_empty());
    let n = rest.next().and_then(|l| l.trim().parse::<usize>().ok());
    match n {
        Some(n) => {
            let strings: Vec<String> = rest.take(n).map(|l| l.trim().to_string()).collect();
            if strings.len() < n {
                out.warnings.push(format!(
                    "line {line}: [relabel] has {} of {n} lines",
                    strings.len()
                ));
            }
            out.ops.push(OverlayOp::Relabel(strings));
        }
        None => out
            .warnings
            .push(format!("line {line}: [relabel] without a count")),
    }
}
