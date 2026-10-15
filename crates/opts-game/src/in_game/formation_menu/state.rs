//! State of the formation editor: the resource, its buttons' actions and what the page shows.

use bevy::prelude::*;
use opts_core::formats::trains::{FormationIssue, TrainFormation};

/// Every entity of the editor page.
#[derive(Component)]
pub(super) struct FormationRoot;

/// A scrollable list of the page; the number says which carried-over offset it uses.
#[derive(Component)]
pub(super) struct FormationList(pub(super) usize);

/// Commands for the editor from outside the UI (`menu.do formation <command>`): `add <text>`,
/// `trains`/`cars` (the list), `preset <text>`, `search <text>`, `remove|up|down|turn|power <n>`
/// (cars counted from 1), `reverse`, `clear`, `entry <text>`, `spawn`.
#[derive(Resource, Default)]
pub struct FormationCommands(pub Vec<String>);

/// Which list the left column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ListKind {
    /// The rail vehicles of the installation.
    #[default]
    Cars,
    /// The `.zug` consists of the installation.
    Presets,
}

/// What a button of the page does.
#[derive(Component, Clone, Debug)]
pub(super) enum Act {
    /// The active tab.
    Noop,
    /// Back to the Select Vehicle page.
    Back,
    List(ListKind),
    Pick(usize),
    PickPreset(usize),
    Add,
    /// Adds the car with this path (the remote `add`).
    AddFile(String),
    Row(usize),
    Up(usize),
    Down(usize),
    Turn(usize),
    Power(usize),
    Remove(usize),
    Reverse,
    Clear,
    Spawn,
    Entry(u32),
}

/// A rail vehicle file of the installation.
#[derive(Debug, Clone)]
pub(super) struct Car {
    /// Path relative to the OMSI root.
    pub(super) rel: String,
    /// What the list shows.
    pub(super) label: String,
    pub(super) mass_t: Option<f64>,
}

/// Numbers shown for the formation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Summary {
    pub(super) parts: usize,
    pub(super) mass_t: f64,
}

/// The state of the editor.
#[derive(Resource)]
pub struct FormationEditor {
    pub(super) formation: TrainFormation,
    pub(super) selected: Option<usize>,
    pub(super) pick: Option<usize>,
    pub(super) list: ListKind,
    pub(super) search: String,
    pub(super) cars: Vec<Car>,
    pub(super) presets: Vec<String>,
    pub(super) lists_loaded: bool,
    pub(super) scroll: [f32; 3],
    pub(super) status: String,
    pub(super) issues: Vec<FormationIssue>,
    pub(super) summary: Summary,
    /// Bumped by every change the page shows.
    pub(super) revision: u64,
    pub(super) validated: u64,
    pub(super) built: Option<u64>,
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
    pub(super) fn touch(&mut self) {
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

    /// Entries of the left column: index and text, filtered by the search words.
    pub(super) fn shown_list(&self) -> Vec<(usize, String)> {
        let words: Vec<String> = self
            .search
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let all: Vec<String> = match self.list {
            ListKind::Cars => self.cars.iter().map(|c| c.label.clone()).collect(),
            ListKind::Presets => self
                .presets
                .iter()
                .map(|p| p.trim_start_matches("Trains/").to_owned())
                .collect(),
        };
        all.into_iter()
            .enumerate()
            .filter(|(_, t)| {
                let hay = t.to_lowercase();
                words.iter().all(|w| hay.contains(w))
            })
            .collect()
    }

    /// One text per car of the train.
    pub(super) fn row_texts(&self) -> Vec<String> {
        self.formation
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = s.vehicle.as_str();
                let name = name.rsplit(['\\', '/']).next().unwrap_or(name);
                let turned = if s.orientation == opts_core::formats::trains::Orientation::Backward {
                    " turned"
                } else {
                    ""
                };
                let power = match s.powered {
                    Some(true) => " pulls",
                    Some(false) => " hauled",
                    None => "",
                };
                format!("{}  {name}{turned}{power}", i + 1)
            })
            .collect()
    }

    pub(super) fn summary_text(&self) -> String {
        format!(
            "{} cars, {} vehicle files, about {:.0} t",
            self.formation.len(),
            self.summary.parts,
            self.summary.mass_t
        )
    }
}
