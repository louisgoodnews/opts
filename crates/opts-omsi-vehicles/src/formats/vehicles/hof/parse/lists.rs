//! The string lists of a `.hof` file: the suite directories, the terminus and the busstop list.

use super::super::*;

pub(super) fn parse_suite_dirs(lines: &[&str], mut i: usize, hof: &mut HofFile) -> usize {
    while i < lines.len() {
        let raw = lines[i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("[hof_suite_directories_end]") {
            return i + 1;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return i;
        }
        if !trimmed.is_empty() {
            hof.suite_directories.push(trimmed.to_string());
        }
        i += 1;
    }
    i
}

pub(super) fn parse_terminus_list(lines: &[&str], mut i: usize, hof: &mut HofFile) -> usize {
    while i < lines.len() {
        let raw = lines[i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("[end]") {
            return i + 1;
        }
        if is_keyword(raw) {
            return i;
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("...") {
            if let Some(term) = parse_terminus_list_row(raw, hof.stringcount_terminus) {
                hof.termini.push(term);
            }
        }
        i += 1;
    }
    i
}

pub(super) fn parse_busstop_list(lines: &[&str], mut i: usize, hof: &mut HofFile) -> usize {
    while i < lines.len() {
        let raw = lines[i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("[end]") {
            return i + 1;
        }
        if is_keyword(raw) {
            return i;
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("...") {
            if let Some(stop) = parse_busstop_list_row(raw, hof.stringcount_busstop) {
                hof.busstops.push(stop);
            }
        }
        i += 1;
    }
    i
}
