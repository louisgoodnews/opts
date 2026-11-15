//! The single `addterminus` and `addbusstop` records of a `.hof` file.

use super::super::*;

pub(super) fn parse_single_terminus(
    lines: &[&str],
    i: usize,
    all_exit: bool,
    hof: &mut HofFile,
) -> usize {
    let Some((code_str, mut next_i)) = next_param(lines, i) else {
        return i;
    };
    let Ok(code) = code_str.parse::<i32>() else {
        return i;
    };
    let mut ident = String::new();
    if let Some((id_str, ni)) = next_param_raw(lines, next_i) {
        ident = id_str.to_string();
        next_i = ni;
    }
    let mut strings = Vec::new();
    let target_count = hof.stringcount_terminus;
    for _ in 0..target_count {
        if next_i < lines.len() {
            let raw = lines[next_i].trim_end_matches(['\r', '\n']);
            if is_keyword(raw) || raw.trim().starts_with("...") {
                break;
            }
            strings.push(raw.to_string());
            next_i += 1;
        }
    }
    while next_i < lines.len() {
        let raw = lines[next_i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.starts_with("...") || trimmed.starts_with('#') {
            next_i += 1;
        } else {
            break;
        }
    }
    while strings.len() < target_count {
        strings.push(String::new());
    }
    hof.termini.push(HofTerminus {
        code,
        ident,
        strings,
        all_exit,
    });
    next_i
}

pub(super) fn parse_single_busstop(lines: &[&str], i: usize, hof: &mut HofFile) -> usize {
    let Some((ident, mut next_i)) = next_param_raw(lines, i) else {
        return i;
    };
    let mut strings = Vec::new();
    let target_count = hof.stringcount_busstop;
    for _ in 0..target_count {
        if next_i < lines.len() {
            let raw = lines[next_i].trim_end_matches(['\r', '\n']);
            if is_keyword(raw) || raw.trim().starts_with("...") {
                break;
            }
            strings.push(raw.to_string());
            next_i += 1;
        }
    }
    while next_i < lines.len() {
        let raw = lines[next_i].trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.starts_with("...") || trimmed.starts_with('#') {
            next_i += 1;
        } else {
            break;
        }
    }
    while strings.len() < target_count {
        strings.push(String::new());
    }
    hof.busstops.push(HofBusStop {
        ident: ident.to_string(),
        strings,
    });
    next_i
}
