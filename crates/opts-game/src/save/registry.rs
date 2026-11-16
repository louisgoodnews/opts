//! The section registry: feature code registers what to capture and how to apply it; the save
//! system calls the functions and never looks inside the data (`docs/src/architecture/session-save.md`).

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::Value;

use super::container::{Fingerprint, SaveFile, SectionBlob};

/// Captures a section from the running session. `Ok(None)` leaves the section out (nothing to say).
pub type CaptureFn = fn(&mut World) -> Result<Option<Value>, String>;
/// Applies a section (already migrated to the registered version) to the session.
pub type ApplyFn = fn(&mut World, &Value) -> Result<(), String>;
/// Turns the data of an older version `from` into data of the registered version.
pub type MigrateFn = fn(from: u32, data: Value) -> Result<Value, String>;

/// What a [`CheckFn`] may look at.
#[derive(Debug, Clone, Copy)]
pub struct CheckEnv<'a> {
    /// The OMSI installation, if one is known.
    pub root: Option<&'a std::path::Path>,
    /// The fingerprint stored in the file.
    pub fingerprint: &'a Fingerprint,
}

/// What kind of problem a section found with the content it refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// The map is not in the installation.
    MissingMap,
    /// A vehicle is not in the installation.
    MissingVehicle,
    /// Something exists but differs from what was saved.
    Changed,
}

/// One problem with the content a save refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// What it is.
    pub kind: IssueKind,
    /// The map or vehicle concerned (display text).
    pub subject: String,
    /// The save cannot be restored because of it.
    pub blocking: bool,
}

/// Looks for content the section's data refers to that is missing or changed.
pub type CheckFn = fn(&Value, &CheckEnv) -> Vec<Issue>;

/// When a section's `apply` runs during a restore (its optional `prepare` always runs first, before
/// the session starts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// When the world is loaded and the HUD is up, before the saved vehicles are rebuilt.
    World,
    /// When every saved vehicle exists (and has stood still for a few frames), before their saved
    /// state is put back: the active vehicle is chosen here, because activating a vehicle parks
    /// the previous one.
    Vehicles,
    /// When every saved vehicle has been rebuilt (or has failed) and has received its state.
    Settled,
}

/// One section of the save.
#[derive(Debug, Clone, Copy)]
pub struct SaveSection {
    /// Key in the file: lower-case words and underscores, never reused for other content.
    pub id: &'static str,
    /// The version `capture` writes and `apply` reads.
    pub version: u32,
    /// When a restore runs `apply`.
    pub stage: Stage,
    /// Order inside the stage, and among the `prepare` functions (lower first).
    pub order: i32,
    /// A failure to apply it stops the load (the session section: without it nothing is known).
    pub required: bool,
    /// Session to JSON.
    pub capture: CaptureFn,
    /// Before the session starts: writes the choices the loading pipeline reads (map, start point,
    /// date, weather). The world does not exist yet.
    pub prepare: Option<ApplyFn>,
    /// JSON to the running session, in `stage`.
    pub apply: Option<ApplyFn>,
    /// Older versions to the current one; without it an older section is skipped with a warning.
    pub migrate: Option<MigrateFn>,
    /// Finds missing or changed content the section refers to (for the save list's warning and
    /// the load's refusal), without touching the world.
    pub check: Option<CheckFn>,
    /// One line for the documentation and `session.saves` (what the section holds).
    pub about: &'static str,
}

/// All sections.
#[derive(Resource, Debug, Default, Clone)]
pub struct SaveRegistry {
    sections: Vec<SaveSection>,
}

impl SaveRegistry {
    /// Adds a section; one with the same id is replaced.
    pub fn register(&mut self, section: SaveSection) {
        self.sections.retain(|s| s.id != section.id);
        self.sections.push(section);
    }

    /// The section with this id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&SaveSection> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// All sections in registration order.
    #[must_use]
    pub fn all(&self) -> &[SaveSection] {
        &self.sections
    }

    /// Captures every section. A failing section is reported in the second part and left out; the
    /// caller decides whether that is fatal (the save of the session section is).
    pub fn capture_all(
        sections: &[SaveSection],
        world: &mut World,
    ) -> (BTreeMap<String, SectionBlob>, Vec<(String, String)>) {
        let mut blobs = BTreeMap::new();
        let mut failures = Vec::new();
        for s in sections {
            match (s.capture)(world) {
                Ok(Some(data)) => {
                    blobs.insert(
                        s.id.to_owned(),
                        SectionBlob {
                            version: s.version,
                            data,
                        },
                    );
                }
                Ok(None) => {}
                Err(e) => failures.push((s.id.to_owned(), e)),
            }
        }
        (blobs, failures)
    }
}

/// `App::register_save_section`.
pub trait SaveAppExt {
    /// Registers a section in the [`SaveRegistry`].
    fn register_save_section(&mut self, section: SaveSection) -> &mut Self;
}

impl SaveAppExt for App {
    fn register_save_section(&mut self, section: SaveSection) -> &mut Self {
        self.world_mut()
            .get_resource_or_insert_with(SaveRegistry::default)
            .register(section);
        self
    }
}

mod restore_plan;
pub use restore_plan::{carry_over_foreign, plan_restore, preflight, RestorePlan};

#[cfg(test)]
mod tests;
