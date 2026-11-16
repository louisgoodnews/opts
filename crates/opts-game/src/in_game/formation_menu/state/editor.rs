//! The state of the formation editor: the resource with its fields, and the changes the page makes
//! to the formation.

use bevy::prelude::*;
use opts_core::formats::trains::{FormationIssue, TrainFormation};

use super::{Car, ListKind, Summary};

/// The state of the editor.
#[derive(Resource)]
pub struct FormationEditor {
    pub(in super::super) formation: TrainFormation,
    pub(in super::super) selected: Option<usize>,
    pub(in super::super) pick: Option<usize>,
    pub(in super::super) list: ListKind,
    pub(in super::super) search: String,
    pub(in super::super) cars: Vec<Car>,
    pub(in super::super) presets: Vec<String>,
    /// Saved formations: id, text, readable.
    pub(in super::super) saved: Vec<(String, String, bool)>,
    /// The saved list needs reading again.
    pub(in super::super) saved_dirty: bool,
    /// The id the train was loaded from or last saved as (Save then replaces that file).
    pub(in super::super) saved_id: Option<String>,
    /// Typing edits the train's name.
    pub(in super::super) renaming: bool,
    pub(in super::super) lists_loaded: bool,
    pub(in super::super) scroll: [f32; 3],
    pub(in super::super) status: String,
    pub(in super::super) issues: Vec<FormationIssue>,
    pub(in super::super) summary: Summary,
    /// Bumped by every change the page shows.
    pub(in super::super) revision: u64,
    pub(in super::super) validated: u64,
    pub(in super::super) built: Option<u64>,
}

impl Default for FormationEditor {
    fn default() -> Self {
        Self {
            formation: TrainFormation::custom("Formation"),
            selected: None,
            pick: None,
            list: ListKind::default(),
            search: String::new(),
            cars: Vec::new(),
            presets: Vec::new(),
            saved: Vec::new(),
            saved_dirty: true,
            saved_id: None,
            renaming: false,
            lists_loaded: false,
            scroll: [0.0; 3],
            status: String::new(),
            issues: Vec::new(),
            summary: Summary::default(),
            revision: 1,
            validated: 0,
            built: None,
        }
    }
}

impl FormationEditor {
    pub(in super::super) fn touch(&mut self) {
        self.revision += 1;
    }

    /// The formation being composed.
    #[must_use]
    pub fn formation(&self) -> &TrainFormation {
        &self.formation
    }

    /// Replaces the formation (a loaded one).
    pub fn set_formation(&mut self, formation: TrainFormation) {
        self.formation = formation;
        self.saved_id = None;
        self.selected = None;
        self.scroll[1] = 0.0;
        self.touch();
    }

    /// Whether *Spawn* is allowed: two cars, no problem, an entry point.
    #[must_use]
    pub fn can_spawn(&self, entry: Option<u32>) -> bool {
        self.formation.len() >= 2
            && self.validated == self.revision
            && self.issues.is_empty()
            && entry.is_some()
    }
}
