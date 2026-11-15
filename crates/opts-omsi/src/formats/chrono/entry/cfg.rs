//! `Chrono.cfg` and `Chrono_<LANG>.dsc` reader: the `[keyword]` lines and the fields they set.

use super::*;

impl ChronoEntry {
    /// Reads the fields of a `Chrono.cfg` (or a `Chrono_<LANG>.dsc`, which has `[name]` and
    /// `[description]`). Unknown keywords are ignored.
    pub fn read_cfg(&mut self, bytes: &[u8]) {
        let text = decode_bytes(bytes);
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let mut i = 0;
        while i < lines.len() {
            let Some(kw) = keyword(lines[i]) else {
                i += 1;
                continue;
            };
            i += 1;
            match kw.as_str() {
                "startdate" => {
                    self.start = next_value(&lines, &mut i).and_then(|v| ChronoDate::parse(&v))
                }
                "enddate" => {
                    self.end = next_value(&lines, &mut i).and_then(|v| ChronoDate::parse(&v))
                }
                "name" => self.name = next_value(&lines, &mut i),
                "moneysystem" => self.moneysystem = next_value(&lines, &mut i),
                "ticketpack" => self.ticketpacks.extend(next_value(&lines, &mut i)),
                "deactivate_lines" => {
                    let n = next_value(&lines, &mut i)
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    for _ in 0..n {
                        match next_value(&lines, &mut i) {
                            Some(v) => self.deactivate_lines.push(v),
                            None => break,
                        }
                    }
                }
                "description" => {
                    let mut text = Vec::new();
                    while i < lines.len() {
                        let k = keyword(lines[i]);
                        if k.as_deref()
                            .is_some_and(|k| k == "end" || CFG_KEYWORDS.contains(&k))
                        {
                            break;
                        }
                        text.push(lines[i].trim_end());
                        i += 1;
                    }
                    let joined = text.join("\n").trim().to_string();
                    self.description = (!joined.is_empty()).then_some(joined);
                }
                _ => {}
            }
        }
    }
}

const CFG_KEYWORDS: [&str; 7] = [
    "startdate",
    "enddate",
    "name",
    "moneysystem",
    "ticketpack",
    "deactivate_lines",
    "description",
];

/// The lower-case keyword of a `[keyword]` line.
fn keyword(line: &str) -> Option<String> {
    let t = line.trim_end();
    (t.len() >= 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_ascii_lowercase())
}

/// The next non-blank line before the next keyword, trimmed.
fn next_value(lines: &[&str], i: &mut usize) -> Option<String> {
    while *i < lines.len() {
        if keyword(lines[*i]).is_some() {
            return None;
        }
        let t = lines[*i].trim();
        *i += 1;
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}
