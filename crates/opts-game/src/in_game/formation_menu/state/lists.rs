//! The lists of the editor: the entries of the left column filtered by the search, the rows of the
//! train, and its summary text.

use super::editor::FormationEditor;
use super::ListKind;

impl FormationEditor {
    /// Entries of the left column: index and text, filtered by the search words.
    pub(in super::super) fn shown_list(&self) -> Vec<(usize, String)> {
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
            ListKind::Saved => self.saved.iter().map(|(_, t, _)| t.clone()).collect(),
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
    pub(in super::super) fn row_texts(&self) -> Vec<String> {
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

    pub(in super::super) fn summary_text(&self) -> String {
        format!(
            "{} cars, {} vehicle files, about {:.0} t",
            self.formation.len(),
            self.summary.parts,
            self.summary.mass_t
        )
    }
}
