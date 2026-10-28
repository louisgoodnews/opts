//! Parser for OMSI depot (`.hof`) files.
//!
//! A `.hof` file holds operational depot data for vehicles: destination codes,
//! display strings, bus stops, routes, and depot global strings.
//! Specification: `docs/src/formats/text/vehicles/hof.md`.

use std::collections::HashMap;

use thiserror::Error;

use crate::formats::text::decode_bytes;

/// Error type for `.hof` depot file parsing.
#[derive(Debug, Error, PartialEq)]
pub enum HofParseError {
    /// File is empty or contains no recognized depot sections.
    #[error("Empty or invalid HOF content")]
    Empty,
}

/// A parsed depot (`.hof`) file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HofFile {
    /// Display name of the depot (`[name]`).
    pub name: String,
    /// Destination string for out-of-service trips (`[servicetrip]`).
    pub service_trip: String,
    /// Global depot string slots (`[global_strings]`).
    pub global_strings: Vec<String>,
    /// Vehicle directory names this depot file applies to (`[HOF_SUITE_DIRECTORIES]`).
    pub suite_directories: Vec<String>,
    /// Number of string columns per terminus row (`stringcount_terminus`).
    pub stringcount_terminus: usize,
    /// Number of string columns per bus stop row (`stringcount_busstop`).
    pub stringcount_busstop: usize,
    /// Terminus / destination list.
    pub termini: Vec<HofTerminus>,
    /// Bus stop list.
    pub busstops: Vec<HofBusStop>,
    /// Route definitions (`[infosystem_trip]` + `[infosystem_busstop_list]`).
    pub routes: Vec<HofRoute>,

    // Quick lookup maps (built after parsing)
    code_to_terminus: HashMap<i32, usize>,
    ident_to_busstop: HashMap<String, usize>,
    route_code_to_route: HashMap<i32, usize>,
}

/// A terminus (destination) entry in a depot file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HofTerminus {
    /// Numeric destination code as keyed on the IBIS.
    pub code: i32,
    /// Internal identifier / display name.
    pub ident: String,
    /// Display and roller blind strings (length matching `stringcount_terminus`).
    pub strings: Vec<String>,
    /// Whether all passengers must exit at this terminus (`[addterminus_allexit]` or `{ALLEX}`).
    pub all_exit: bool,
}

/// A bus stop entry in a depot file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HofBusStop {
    /// Lookup key used by routes.
    pub ident: String,
    /// Stop display strings (length matching `stringcount_busstop`).
    pub strings: Vec<String>,
}

/// A route (trip) entry in a depot file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HofRoute {
    /// Route code as keyed on the IBIS (e.g. `13001` for Line 130 Route 01).
    pub route_code: i32,
    /// Route name string (e.g. `URUH-NERV`).
    pub route_string: String,
    /// Destination code pointing to a terminus.
    pub terminus_code: i32,
    /// Line string shown on line displays (e.g. `130`).
    pub line_string: String,
    /// Ordered list of bus stop idents along the route.
    pub busstops: Vec<String>,
}

mod parse;

impl HofFile {
    /// Rebuilds internal lookup maps for fast indexing.
    pub fn build_indices(&mut self) {
        self.code_to_terminus.clear();
        for (idx, term) in self.termini.iter().enumerate() {
            self.code_to_terminus.entry(term.code).or_insert(idx);
        }

        self.ident_to_busstop.clear();
        for (idx, stop) in self.busstops.iter().enumerate() {
            self.ident_to_busstop
                .entry(stop.ident.to_ascii_lowercase())
                .or_insert(idx);
        }

        self.route_code_to_route.clear();
        for (idx, route) in self.routes.iter().enumerate() {
            self.route_code_to_route
                .entry(route.route_code)
                .or_insert(idx);
        }
    }

    /// Finds a terminus by numeric destination code, returning `(row_index, terminus)`.
    #[must_use]
    pub fn find_terminus_by_code(&self, code: i32) -> Option<(usize, &HofTerminus)> {
        self.code_to_terminus
            .get(&code)
            .copied()
            .and_then(|idx| self.termini.get(idx).map(|t| (idx, t)))
    }

    /// Gets a terminus by row index.
    #[must_use]
    pub fn terminus(&self, index: usize) -> Option<&HofTerminus> {
        self.termini.get(index)
    }

    /// Gets the string column `col` of terminus at `index`.
    #[must_use]
    pub fn terminus_string(&self, index: usize, col: usize) -> Option<&str> {
        self.termini
            .get(index)
            .and_then(|t| t.strings.get(col).map(String::as_str))
    }

    /// Finds a bus stop by ident (case-insensitive), returning `(row_index, stop)`.
    #[must_use]
    pub fn find_busstop_by_ident(&self, ident: &str) -> Option<(usize, &HofBusStop)> {
        self.ident_to_busstop
            .get(&ident.to_ascii_lowercase())
            .copied()
            .and_then(|idx| self.busstops.get(idx).map(|s| (idx, s)))
    }

    /// Gets a bus stop by row index.
    #[must_use]
    pub fn busstop(&self, index: usize) -> Option<&HofBusStop> {
        self.busstops.get(index)
    }

    /// Gets the string column `col` of bus stop at `index`.
    #[must_use]
    pub fn busstop_string(&self, index: usize, col: usize) -> Option<&str> {
        self.busstops
            .get(index)
            .and_then(|s| s.strings.get(col).map(String::as_str))
    }

    /// Finds a route by route code (e.g. `13001`), returning `(index, route)`.
    #[must_use]
    pub fn find_route_by_code(&self, route_code: i32) -> Option<(usize, &HofRoute)> {
        self.route_code_to_route
            .get(&route_code)
            .copied()
            .and_then(|idx| self.routes.get(idx).map(|r| (idx, r)))
    }

    /// Gets a route by index.
    #[must_use]
    pub fn route(&self, index: usize) -> Option<&HofRoute> {
        self.routes.get(index)
    }

    /// Gets a depot global string slot (0: announcements, 1: roller-blind, 2: side-signs, etc.).
    #[must_use]
    pub fn global_string(&self, slot: usize) -> Option<&str> {
        self.global_strings.get(slot).map(String::as_str)
    }
}

/// Checks whether `line` starts at column 0 with a `[` keyword.
fn is_keyword(line: &str) -> bool {
    let trimmed = line.trim_end_matches(['\r', '\n', ' ', '\t']);
    trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2
}

/// Returns the next non-empty, non-comment parameter line.
fn next_param<'a>(lines: &'a [&'a str], mut i: usize) -> Option<(&'a str, usize)> {
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
fn next_param_raw<'a>(lines: &'a [&'a str], mut i: usize) -> Option<(&'a str, usize)> {
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
fn parse_terminus_list_row(row: &str, stringcount: usize) -> Option<HofTerminus> {
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
fn parse_busstop_list_row(row: &str, stringcount: usize) -> Option<HofBusStop> {
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

#[cfg(test)]
mod tests;
