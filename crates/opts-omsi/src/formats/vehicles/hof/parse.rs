//! Line and section parser for OMSI `.hof` depot files.

use super::*;

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

            // Column-0 bracketed keyword
            if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
                let kw = line[1..line.len() - 1].trim().to_ascii_lowercase();
                i += 1;
                match kw.as_str() {
                    "name" => {
                        if let Some((val, next_i)) = next_param(&lines, i) {
                            hof.name = val.to_string();
                            i = next_i;
                        }
                    }
                    "servicetrip" => {
                        if let Some((val, next_i)) = next_param(&lines, i) {
                            hof.service_trip = val.to_string();
                            i = next_i;
                        }
                    }
                    "global_strings" => {
                        if let Some((count_str, mut next_i)) = next_param(&lines, i) {
                            if let Ok(count) = count_str.parse::<usize>() {
                                hof.global_strings.clear();
                                for _ in 0..count {
                                    if let Some((s, ni)) = next_param_raw(&lines, next_i) {
                                        hof.global_strings.push(s.to_string());
                                        next_i = ni;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            i = next_i;
                        }
                    }
                    "hof_suite_directories" => {
                        while i < lines.len() {
                            let raw = lines[i].trim_end_matches(['\r', '\n']);
                            let trimmed = raw.trim();
                            if trimmed.eq_ignore_ascii_case("[hof_suite_directories_end]") {
                                i += 1;
                                break;
                            }
                            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                                // Another block started unexpectedly
                                break;
                            }
                            if !trimmed.is_empty() {
                                hof.suite_directories.push(trimmed.to_string());
                            }
                            i += 1;
                        }
                    }
                    "addterminus_list" => {
                        while i < lines.len() {
                            let raw = lines[i].trim_end_matches(['\r', '\n']);
                            let trimmed = raw.trim();
                            if trimmed.eq_ignore_ascii_case("[end]") {
                                i += 1;
                                break;
                            }
                            if is_keyword(raw) {
                                break;
                            }
                            if !trimmed.is_empty()
                                && !trimmed.starts_with('#')
                                && !trimmed.starts_with("...")
                            {
                                if let Some(term) =
                                    parse_terminus_list_row(raw, hof.stringcount_terminus)
                                {
                                    hof.termini.push(term);
                                }
                            }
                            i += 1;
                        }
                    }
                    "addbusstop_list" => {
                        while i < lines.len() {
                            let raw = lines[i].trim_end_matches(['\r', '\n']);
                            let trimmed = raw.trim();
                            if trimmed.eq_ignore_ascii_case("[end]") {
                                i += 1;
                                break;
                            }
                            if is_keyword(raw) {
                                break;
                            }
                            if !trimmed.is_empty()
                                && !trimmed.starts_with('#')
                                && !trimmed.starts_with("...")
                            {
                                if let Some(stop) =
                                    parse_busstop_list_row(raw, hof.stringcount_busstop)
                                {
                                    hof.busstops.push(stop);
                                }
                            }
                            i += 1;
                        }
                    }
                    "addterminus" | "addterminus_allexit" => {
                        let all_exit = kw == "addterminus_allexit";
                        if let Some((code_str, mut next_i)) = next_param(&lines, i) {
                            if let Ok(code) = code_str.parse::<i32>() {
                                let mut ident = String::new();
                                if let Some((id_str, ni)) = next_param_raw(&lines, next_i) {
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
                                i = next_i;
                            }
                        }
                    }
                    "addbusstop" => {
                        if let Some((ident, mut next_i)) = next_param_raw(&lines, i) {
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
                            i = next_i;
                        }
                    }
                    "infosystem_trip" => {
                        let mut trip_i = i;
                        if let Some((c_str, n1)) = next_param(&lines, trip_i) {
                            let route_code = c_str.parse::<i32>().unwrap_or(0);
                            let (route_str, n2) = next_param(&lines, n1).unwrap_or(("", n1));
                            let (t_code_str, n3) = next_param(&lines, n2).unwrap_or(("", n2));
                            let terminus_code = t_code_str.parse::<i32>().unwrap_or(0);
                            let (line_str, n4) = next_param(&lines, n3).unwrap_or(("", n3));
                            trip_i = n4;

                            // Scan forward for the matching [infosystem_busstop_list]
                            let mut busstops = Vec::new();
                            while trip_i < lines.len() {
                                let raw = lines[trip_i].trim_end_matches(['\r', '\n']);
                                if is_keyword(raw) {
                                    let k = raw.trim()[1..raw.trim().len() - 1]
                                        .trim()
                                        .to_ascii_lowercase();
                                    if k == "infosystem_busstop_list" {
                                        trip_i += 1;
                                        if let Some((cnt_str, mut stop_i)) =
                                            next_param(&lines, trip_i)
                                        {
                                            if let Ok(count) = cnt_str.parse::<usize>() {
                                                for _ in 0..count {
                                                    if let Some((stop_ident, next_stop_i)) =
                                                        next_param_raw(&lines, stop_i)
                                                    {
                                                        // Sheets export empty trailing columns as tabs: the ident is the first column.
                                                        busstops.push(
                                                            stop_ident
                                                                .split('\t')
                                                                .next()
                                                                .unwrap_or_default()
                                                                .to_string(),
                                                        );
                                                        stop_i = next_stop_i;
                                                    }
                                                }
                                            }
                                            trip_i = stop_i;
                                        }
                                        break;
                                    } else {
                                        break;
                                    }
                                }
                                trip_i += 1;
                            }

                            hof.routes.push(HofRoute {
                                route_code,
                                route_string: route_str.to_string(),
                                terminus_code,
                                line_string: line_str.to_string(),
                                busstops,
                            });
                            i = trip_i;
                        }
                    }
                    _ => {
                        // Other keywords ignored
                    }
                }
            } else {
                // Check bare keywords
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("stringcount_terminus") {
                    i += 1;
                    if let Some((count_str, next_i)) = next_param(&lines, i) {
                        if let Ok(count) = count_str.parse::<usize>() {
                            hof.stringcount_terminus = count;
                        }
                        i = next_i;
                    }
                } else if trimmed.eq_ignore_ascii_case("stringcount_busstop") {
                    i += 1;
                    if let Some((count_str, next_i)) = next_param(&lines, i) {
                        if let Ok(count) = count_str.parse::<usize>() {
                            hof.stringcount_busstop = count;
                        }
                        i = next_i;
                    }
                } else {
                    i += 1;
                }
            }
        }

        hof.build_indices();
        Ok(hof)
    }
}
