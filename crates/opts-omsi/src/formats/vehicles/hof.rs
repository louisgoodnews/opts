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

mod rows;

use rows::{is_keyword, next_param, next_param_raw, parse_busstop_list_row, parse_terminus_list_row};

#[cfg(test)]
mod tests;
