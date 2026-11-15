//! The `infosystem_trip` records of a `.hof` file: a route with its busstops.

use super::super::*;

pub(super) fn parse_trip_busstops(lines: &[&str], mut trip_i: usize) -> (Vec<String>, usize) {
    let mut busstops = Vec::new();
    while trip_i < lines.len() {
        let raw = lines[trip_i].trim_end_matches(['\r', '\n']);
        if is_keyword(raw) {
            let k = raw.trim()[1..raw.trim().len() - 1]
                .trim()
                .to_ascii_lowercase();
            if k == "infosystem_busstop_list" {
                trip_i += 1;
                if let Some((cnt_str, mut stop_i)) = next_param(lines, trip_i) {
                    if let Ok(count) = cnt_str.parse::<usize>() {
                        for _ in 0..count {
                            if let Some((stop_ident, next_stop_i)) = next_param_raw(lines, stop_i) {
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
            }
            break;
        }
        trip_i += 1;
    }
    (busstops, trip_i)
}

pub(super) fn parse_infosystem_trip(lines: &[&str], i: usize, hof: &mut HofFile) -> usize {
    let Some((c_str, n1)) = next_param(lines, i) else {
        return i;
    };
    let route_code = c_str.parse::<i32>().unwrap_or(0);
    let (route_str, n2) = next_param(lines, n1).unwrap_or(("", n1));
    let (t_code_str, n3) = next_param(lines, n2).unwrap_or(("", n2));
    let terminus_code = t_code_str.parse::<i32>().unwrap_or(0);
    let (line_str, n4) = next_param(lines, n3).unwrap_or(("", n3));

    let (busstops, trip_i) = parse_trip_busstops(lines, n4);
    hof.routes.push(HofRoute {
        route_code,
        route_string: route_str.to_string(),
        terminus_code,
        line_string: line_str.to_string(),
        busstops,
    });
    trip_i
}
