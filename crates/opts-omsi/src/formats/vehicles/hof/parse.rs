//! Line and section parser for OMSI `.hof` depot files.

use super::*;

mod entries;
mod lists;
mod routes;

use entries::{parse_single_busstop, parse_single_terminus};
use lists::{parse_busstop_list, parse_suite_dirs, parse_terminus_list};
use routes::parse_infosystem_trip;

fn parse_name(lines: &[&str], i: usize, target: &mut String) -> usize {
    if let Some((val, next_i)) = next_param(lines, i) {
        *target = val.to_string();
        next_i
    } else {
        i
    }
}

fn parse_global_strings(lines: &[&str], i: usize, hof: &mut HofFile) -> usize {
    if let Some((count_str, mut next_i)) = next_param(lines, i) {
        if let Ok(count) = count_str.parse::<usize>() {
            hof.global_strings.clear();
            for _ in 0..count {
                if let Some((s, ni)) = next_param_raw(lines, next_i) {
                    hof.global_strings.push(s.to_string());
                    next_i = ni;
                } else {
                    break;
                }
            }
        }
        next_i
    } else {
        i
    }
}

fn handle_bracket_keyword(kw: &str, lines: &[&str], i: usize, hof: &mut HofFile) -> usize {
    match kw {
        "name" => parse_name(lines, i, &mut hof.name),
        "servicetrip" => parse_name(lines, i, &mut hof.service_trip),
        "global_strings" => parse_global_strings(lines, i, hof),
        "hof_suite_directories" => parse_suite_dirs(lines, i, hof),
        "addterminus_list" => parse_terminus_list(lines, i, hof),
        "addbusstop_list" => parse_busstop_list(lines, i, hof),
        "addterminus" => parse_single_terminus(lines, i, false, hof),
        "addterminus_allexit" => parse_single_terminus(lines, i, true, hof),
        "addbusstop" => parse_single_busstop(lines, i, hof),
        "infosystem_trip" => parse_infosystem_trip(lines, i, hof),
        _ => i,
    }
}

fn handle_bare_keyword(trimmed: &str, lines: &[&str], i: usize, hof: &mut HofFile) -> usize {
    if trimmed.eq_ignore_ascii_case("stringcount_terminus") {
        if let Some((count_str, next_i)) = next_param(lines, i + 1) {
            if let Ok(count) = count_str.parse::<usize>() {
                hof.stringcount_terminus = count;
            }
            return next_i;
        }
    } else if trimmed.eq_ignore_ascii_case("stringcount_busstop") {
        if let Some((count_str, next_i)) = next_param(lines, i + 1) {
            if let Ok(count) = count_str.parse::<usize>() {
                hof.stringcount_busstop = count;
            }
            return next_i;
        }
    }
    i + 1
}

impl HofFile {
    /// Parses a `.hof` file from raw bytes (supporting UTF-8 and CP1252 fallbacks).
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, HofParseError> {
        let text = decode_bytes(bytes);
        Self::parse_str(&text)
    }

    /// Parses a `.hof` file from a string slice.
    pub fn parse_str(text: &str) -> Result<Self, HofParseError> {
        let mut hof = HofFile::default();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim_end_matches(['\r', '\n', ' ', '\t']);

            if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
                let kw = line[1..line.len() - 1].trim().to_ascii_lowercase();
                i = handle_bracket_keyword(&kw, &lines, i + 1, &mut hof);
            } else {
                i = handle_bare_keyword(line.trim(), &lines, i, &mut hof);
            }
        }

        hof.build_indices();
        Ok(hof)
    }
}
