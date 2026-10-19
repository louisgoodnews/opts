//! Chrono data files: the group-level update of `ailists.cfg` by `ailists_#upd.cfg`.

/// A replaceable unit of an AI list file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// Lines between units (comments, separators), kept as they are.
    Text(Vec<String>),
    /// `[aigroup_2]` … `[end]` (`depot == false`) or `[aigroup_depot]` with the
    /// `[aigroup_depot_typgroup*]` … `[end]` blocks that follow it (`depot == true`).
    Unit {
        depot: bool,
        name: String,
        lines: Vec<String>,
    },
}

fn keyword(line: &str) -> Option<String> {
    let t = line.trim();
    (t.len() >= 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_ascii_lowercase())
}

/// Index of the `[end]` that closes the block opened at `start` (or of the last line before the
/// next unit keyword, when `[end]` is missing).
fn block_end(lines: &[&str], start: usize) -> usize {
    let mut j = start + 1;
    while j < lines.len() {
        match keyword(lines[j]).as_deref() {
            Some("end") => return j,
            Some(k) if k.starts_with("aigroup") => return j - 1,
            _ => j += 1,
        }
    }
    lines.len() - 1
}

/// The first non-blank line after `start`, trimmed.
fn name_after(lines: &[&str], start: usize) -> String {
    lines[start + 1..]
        .iter()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .filter(|l| keyword(l).is_none())
        .unwrap_or_default()
        .to_string()
}

fn segments(text: &str) -> Vec<Segment> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut out = Vec::new();
    let mut text_run: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let kw = keyword(lines[i]);
        let end = match kw.as_deref() {
            Some("aigroup_2") => Some((false, block_end(&lines, i))),
            Some("aigroup_depot") => {
                // The header holds two lines (depot name, `.hof`); then type groups.
                let mut last = i;
                let mut seen = 0;
                let mut j = i + 1;
                while j < lines.len() && seen < 2 && keyword(lines[j]).is_none() {
                    if !lines[j].trim().is_empty() {
                        seen += 1;
                        last = j;
                    }
                    j += 1;
                }
                loop {
                    let next = (last + 1..lines.len()).find(|&k| keyword(lines[k]).is_some());
                    match next {
                        Some(k)
                            if keyword(lines[k])
                                .is_some_and(|w| w.starts_with("aigroup_depot_typgroup")) =>
                        {
                            last = block_end(&lines, k);
                        }
                        _ => break,
                    }
                }
                Some((true, last))
            }
            _ => None,
        };
        match end {
            Some((depot, j)) => {
                if !text_run.is_empty() {
                    out.push(Segment::Text(std::mem::take(&mut text_run)));
                }
                out.push(Segment::Unit {
                    depot,
                    name: name_after(&lines, i),
                    lines: lines[i..=j].iter().map(|s| (*s).to_string()).collect(),
                });
                i = j + 1;
            }
            None => {
                text_run.push(lines[i].to_string());
                i += 1;
            }
        }
    }
    if !text_run.is_empty() {
        out.push(Segment::Text(text_run));
    }
    out
}

/// Applies an `ailists_#upd.cfg` to the text of an `ailists.cfg`: every `[aigroup_2]` group of
/// the update replaces the base group of the same name (case-insensitive), every
/// `[aigroup_depot]` (with its type groups) the base depot of the same depot name; units the
/// base lacks are appended. Everything else of the base stays as it is. Lines are joined with
/// CRLF.
#[must_use]
pub fn merge_ailists(base: &str, update: &str) -> String {
    let mut merged = segments(base);
    for seg in segments(update) {
        let Segment::Unit { depot, name, lines } = seg else {
            continue;
        };
        let slot = merged.iter_mut().find(|s| {
            matches!(s, Segment::Unit { depot: d, name: n, .. } if *d == depot && n.eq_ignore_ascii_case(&name))
        });
        match slot {
            Some(Segment::Unit { lines: l, .. }) => *l = lines,
            _ => {
                merged.push(Segment::Text(vec![String::new()]));
                merged.push(Segment::Unit { depot, name, lines });
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    for s in merged {
        match s {
            Segment::Text(l) | Segment::Unit { lines: l, .. } => out.extend(l),
        }
    }
    out.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::map::AiLists;

    const BASE: &str = "header comment\r\n\r\n[aigroup_2]\r\nNormalCars\r\n\r\nvehicles\\a.bus\t5\r\n[end]\r\n\r\n***\r\n[aigroup_depot]\r\nHof Spandau\r\nSpandau 1986\r\n\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\SD77.bus\r\n2701\r\n[end]\r\n\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\SD81.bus\r\n2801\r\n[end]\r\n\r\n[aigroup_2]\r\nTrucks\r\nvehicles\\t.ovh\t1\r\n[end]\r\n";

    /// Groups replace by group name, depots by depot name, new units are appended, the rest of
    /// the base stays.
    #[test]
    fn update_replaces_groups_and_depots_by_name() {
        let update = "[aigroup_2]\r\nnormalcars\r\n\r\nvehicles\\golf.bus\t7\r\n[end]\r\n\r\n[aigroup_depot]\r\nHof Spandau\r\nSpandau 1990\r\n\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\EN92.bus\r\n6100\r\n[end]\r\n\r\n[aigroup_2]\r\nGDRCars\r\nvehicles\\trabi.ovh\t3\r\n[end]\r\n";
        let merged = merge_ailists(BASE, update);
        assert!(merged.starts_with("header comment"));
        let lists = AiLists::parse(merged.as_bytes());
        let names: Vec<&str> = lists.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["normalcars", "Trucks", "GDRCars"]);
        assert_eq!(lists.groups[0].entries[0].vehicle, "vehicles\\golf.bus");
        assert!(merged.contains("Spandau 1990") && merged.contains("6100"));
        assert!(
            !merged.contains("Spandau 1986") && !merged.contains("SD81"),
            "{merged}"
        );
        assert!(merged.contains("***"));
        // The depot block was replaced in place: before Trucks.
        assert!(merged.find("6100").unwrap() < merged.find("Trucks").unwrap());
    }

    #[test]
    fn an_empty_update_keeps_the_base() {
        let merged = merge_ailists(BASE, "");
        assert_eq!(
            AiLists::parse(merged.as_bytes()),
            AiLists::parse(BASE.as_bytes())
        );
    }
}
