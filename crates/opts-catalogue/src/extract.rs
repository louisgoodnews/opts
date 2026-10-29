//! Entity detection, display metadata and reference extraction.
//!
//! Marker rules follow `catalogue-schema.md` (Stage 2/Part C) and vehicle playability follows
//! `playability-rules.md` (rules R1–R8).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use opts_core::formats::map::MapManifest;
use opts_core::formats::scenery::{SceneryObject, SplineProfile};
use opts_core::formats::text::{decode_bytes, scan_blocks};
use opts_core::formats::vehicles::BusManifest;
use rayon::prelude::*;

use crate::scan::{DirRec, FileRec};

/// Display metadata attached to an entity.
#[derive(Debug, Clone)]
pub enum Info {
    /// No extra metadata.
    None,
    /// Map metadata from `global.cfg`.
    Map {
        /// Display name.
        friendly_name: String,
        /// Description text.
        description: String,
        /// Number of `[map]` tile entries.
        tile_count: i64,
        /// Default depot.
        standard_depot: String,
        /// Year range text.
        years: String,
        /// Entry point names (newline separated, de-duplicated).
        entry_points: String,
    },
    /// Vehicle metadata from `.bus` / `.ovh`.
    Vehicle {
        /// Manufacturer.
        manufacturer: String,
        /// Model.
        model: String,
        /// Default livery.
        livery: String,
        /// Description text.
        description: String,
        /// Curb mass.
        mass_tonnes: Option<f64>,
    },
}

/// An entity found by the marker rules, before it is written to the database.
#[derive(Debug, Clone)]
pub struct EntityDraft {
    /// One of the 16 categories.
    pub category: &'static str,
    /// `"folder"` or `"file"`.
    pub mode: &'static str,
    /// Display name.
    pub name: String,
    /// Marker file's relative path.
    pub marker: String,
    /// Entity directory (relative path).
    pub dir: String,
    /// Player-selectable vehicle.
    pub is_playable: bool,
    /// `road` / `rail` / `aircraft` for vehicles.
    pub vehicle_class: Option<&'static str>,
    /// Marker of the primary entity if this is an alias twin.
    pub alias_of_marker: Option<String>,
    /// Marker mtime.
    pub mtime: i64,
    /// Display metadata.
    pub info: Info,
    /// `(role, raw_reference, root-relative candidate paths in search order)`.
    pub refs: Vec<(&'static str, String, Vec<String>)>,
}

fn norm(rel: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in rel.split(['\\', '/']) {
        match c.trim() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn under(dir: &str, rel: &str) -> String {
    norm(&format!("{dir}/{rel}"))
}

fn first_param(blocks: &[opts_core::formats::text::OmsiBlock], key: &str) -> Option<String> {
    blocks
        .iter()
        .find(|b| b.keyword.eq_ignore_ascii_case(key))
        .and_then(|b| b.parameters.first())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(super) fn stem(name: &str) -> String {
    name.rsplit_once('.').map_or(name, |(s, _)| s).to_string()
}

mod files;
mod maps;
mod refs;
mod vehicle;

use files::{detect_file_entities, detect_money};
use maps::detect_maps;
use vehicle::detect_vehicles;

pub(super) fn friendly_name(bytes: &[u8]) -> Option<String> {
    let blocks = scan_blocks(&decode_bytes(bytes));
    first_param(&blocks, "friendlyname")
}

/// Detects all entities and extracts their metadata. `root` is read for marker contents.
#[must_use]
pub fn detect_entities(root: &Path, files: &[FileRec], dirs: &[DirRec]) -> Vec<EntityDraft> {
    let by_rel: HashMap<String, &FileRec> = files
        .iter()
        .map(|f| (f.rel_path.to_lowercase(), f))
        .collect();
    let read = |rel: &str| fs::read(root.join(rel)).ok();
    let mut out: Vec<EntityDraft> = Vec::new();

    out.extend(detect_vehicles(files, &by_rel, &read));
    out.extend(detect_maps(files, &read));
    out.extend(detect_money(files));
    out.extend(detect_file_entities(files, &read));

    let _ = dirs;
    out
}

#[cfg(test)]
mod tests;
