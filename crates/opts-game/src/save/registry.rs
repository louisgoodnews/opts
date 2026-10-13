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

/// What a restore will do with a file, decided before anything is applied.
#[derive(Debug, Clone, Default)]
pub struct RestorePlan {
    /// `(section, data at the registered version)` ordered by stage, then `order`.
    pub steps: Vec<(SaveSection, Value)>,
    /// Section ids in the file that this build does not know (kept, ignored).
    pub unknown: Vec<String>,
    /// Sections the file lacks (they keep their defaults).
    pub missing: Vec<String>,
    /// Problems that did not stop the plan: a section from a newer version, a failed migration.
    pub warnings: Vec<String>,
}

/// Matches the sections of `file` with the registry: migrates older ones, sets aside unknown,
/// newer and unmigratable ones.
#[must_use]
pub fn plan_restore(sections: &[SaveSection], file: &SaveFile) -> RestorePlan {
    let mut plan = RestorePlan::default();
    for s in sections {
        let Some(blob) = file.sections.get(s.id) else {
            plan.missing.push(s.id.to_owned());
            continue;
        };
        let data = if blob.version == s.version {
            Ok(blob.data.clone())
        } else if blob.version > s.version {
            Err(format!(
                "section `{}` is version {} but this build reads {}: ignored",
                s.id, blob.version, s.version
            ))
        } else if let Some(migrate) = s.migrate {
            migrate(blob.version, blob.data.clone())
                .map_err(|e| format!("section `{}` v{} cannot be migrated: {e}", s.id, blob.version))
        } else {
            Err(format!(
                "section `{}` is version {} and has no migration to {}: ignored",
                s.id, blob.version, s.version
            ))
        };
        match data {
            Ok(d) => plan.steps.push((*s, d)),
            Err(w) => {
                plan.warnings.push(w);
            }
        }
    }
    plan.unknown = file
        .sections
        .keys()
        .filter(|k| !sections.iter().any(|s| s.id == k.as_str()))
        .cloned()
        .collect();
    plan.steps.sort_by_key(|(s, _)| (s.stage, s.order));
    plan
}

/// Every content problem the sections of `file` report.
#[must_use]
pub fn preflight(sections: &[SaveSection], file: &SaveFile, env: &CheckEnv) -> Vec<Issue> {
    plan_restore(sections, file)
        .steps
        .iter()
        .filter_map(|(s, data)| s.check.map(|check| check(data, env)))
        .flatten()
        .collect()
}

impl RestorePlan {
    /// The steps that have a `prepare`, in `order`.
    #[must_use]
    pub fn prepare_steps(&self) -> Vec<&(SaveSection, Value)> {
        let mut steps: Vec<_> = self.steps.iter().filter(|(s, _)| s.prepare.is_some()).collect();
        steps.sort_by_key(|(s, _)| s.order);
        steps
    }
}

/// Copies the sections of `old` that this build did not capture into `new` (unknown ones and
/// newer versions of known ones), so overwriting a save never destroys another build's data.
pub fn carry_over_foreign(sections: &[SaveSection], old: &SaveFile, new: &mut SaveFile) {
    for (id, blob) in &old.sections {
        let known = sections.iter().find(|s| s.id == id.as_str());
        let foreign = known.is_none_or(|s| blob.version > s.version);
        if foreign && !new.sections.contains_key(id) {
            new.sections.insert(id.clone(), blob.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::container::SaveMeta;
    use serde_json::json;

    #[allow(clippy::unnecessary_wraps)]
    fn noop_capture(_: &mut World) -> Result<Option<Value>, String> {
        Ok(Some(json!({"v": 2})))
    }
    fn noop_apply(_: &mut World, _: &Value) -> Result<(), String> {
        Ok(())
    }
    fn up(from: u32, data: Value) -> Result<Value, String> {
        if from == 1 {
            Ok(json!({"v": data["old"].as_i64().unwrap_or(0) + 1}))
        } else {
            Err("unknown version".into())
        }
    }

    fn section(id: &'static str, version: u32, migrate: Option<MigrateFn>) -> SaveSection {
        SaveSection {
            id,
            version,
            stage: Stage::World,
            order: 0,
            required: false,
            capture: noop_capture,
            prepare: None,
            apply: Some(noop_apply),
            migrate,
            check: None,
            about: "",
        }
    }

    fn file_with(sections: &[(&str, u32, Value)]) -> SaveFile {
        let mut f = SaveFile::new(SaveMeta::default());
        for (id, v, d) in sections {
            f.sections.insert(
                (*id).to_owned(),
                SectionBlob {
                    version: *v,
                    data: d.clone(),
                },
            );
        }
        f
    }

    #[test]
    fn the_plan_migrates_old_sections_and_sets_the_rest_aside() {
        let sections = [
            section("same", 2, None),
            section("old", 2, Some(up)),
            section("old_no_migration", 2, None),
            section("too_new", 2, Some(up)),
            section("absent", 1, None),
            section("broken", 5, Some(up)),
        ];
        let file = file_with(&[
            ("same", 2, json!({"v": 7})),
            ("old", 1, json!({"old": 4})),
            ("old_no_migration", 1, json!({})),
            ("too_new", 3, json!({})),
            ("broken", 4, json!({})),
            ("from_the_future", 1, json!({"x": 1})),
        ]);
        let plan = plan_restore(&sections, &file);
        let applied: Vec<_> = plan.steps.iter().map(|(s, d)| (s.id, d.clone())).collect();
        assert_eq!(
            applied,
            vec![("same", json!({"v": 7})), ("old", json!({"v": 5}))]
        );
        assert_eq!(plan.unknown, ["from_the_future"]);
        assert_eq!(plan.missing, ["absent"]);
        assert_eq!(plan.warnings.len(), 3);
    }

    #[test]
    fn steps_follow_stage_then_order() {
        let mut a = section("a", 1, None);
        a.stage = Stage::Settled;
        let b = section("b", 1, None);
        let mut c = section("c", 1, None);
        c.order = -1;
        let file = file_with(&[("a", 1, json!({})), ("b", 1, json!({})), ("c", 1, json!({}))]);
        let plan = plan_restore(&[a, b, c], &file);
        assert_eq!(
            plan.steps.iter().map(|(s, _)| s.id).collect::<Vec<_>>(),
            ["c", "b", "a"]
        );
    }

    #[test]
    fn overwriting_keeps_what_this_build_does_not_understand() {
        let sections = [section("known", 1, None)];
        let old = file_with(&[
            ("known", 1, json!({"old": true})),
            ("future_section", 1, json!({"keep": 1})),
            ("newer_known", 9, json!({})),
        ]);
        let mut new = file_with(&[("known", 1, json!({"new": true}))]);
        carry_over_foreign(&sections, &old, &mut new);
        assert_eq!(new.sections["known"].data, json!({"new": true}));
        assert_eq!(new.sections["future_section"].data, json!({"keep": 1}));
        assert!(new.sections.contains_key("newer_known"));

        let sections = [section("known", 1, None), section("newer_known", 9, None)];
        let mut again = file_with(&[]);
        carry_over_foreign(&sections, &old, &mut again);
        assert!(!again.sections.contains_key("newer_known"), "known and not newer: not foreign");
    }
}
