//! Language packs in the localisation: scanning the user folder, the language list and the
//! per-language report. The pure parts (format, checks) live in `opts_l10n`.

use std::collections::HashMap;
use std::path::Path;

use opts_l10n::check::{Issue, Reference};
use opts_l10n::fluent::Table;
use opts_l10n::pack;

use super::pseudo::PSEUDO_TAG;
use super::{report::LoadedPack, Localization};

impl Localization {
    /// The English source as a reference for checks (all tables the application has added).
    #[must_use]
    pub fn reference(&self) -> Reference {
        Reference::from_table(
            self.bundles.get("en").cloned().unwrap_or_default(),
            HashMap::new(),
        )
    }

    /// Scans `root` (`<data>/lang`) for packs, checks them against the English table and installs
    /// their messages (replacing those of an earlier scan). Never fails: a broken pack or file
    /// shows up in [`Self::pack_problems`] and the language list instead. Call it after every
    /// plugin has added its English messages. Returns the number of usable packs.
    pub fn load_packs(&mut self, root: &Path) -> usize {
        let reference = self.reference();
        let packs = pack::discover(root, &reference);
        self.overlay.clear();
        self.packs.clear();
        for p in packs {
            if let Some(meta) = &p.meta {
                if meta.code != PSEUDO_TAG {
                    self.overlay.insert(meta.code.clone(), p.table.clone());
                }
            }
            self.packs.push(LoadedPack { pack: p });
        }
        self.packs.iter().filter(|p| p.pack.usable()).count()
    }

    /// Drops all pack messages (the built-in tables remain).
    pub fn clear_packs(&mut self) {
        self.overlay.clear();
        self.packs.clear();
    }

    /// Findings of the last scan for folders that cannot be used (no valid `pack.json`), as
    /// `(folder, issues)`.
    #[must_use]
    pub fn pack_problems(&self) -> Vec<(String, Vec<Issue>)> {
        self.packs
            .iter()
            .filter(|p| !p.pack.usable())
            .map(|p| (p.pack.folder.clone(), p.pack.issues.clone()))
            .collect()
    }

    /// The own table of `code` with a pack's messages laid over the built-in ones.
    fn effective(&self, code: &str) -> Table {
        let mut t = self.bundles.get(code).cloned().unwrap_or_default();
        if let Some(o) = self.overlay.get(code) {
            for k in &o.order {
                if !t.messages.contains_key(k) {
                    t.order.push(k.clone());
                }
            }
            t.messages.extend(o.messages.clone());
        }
        t
    }
}

mod languages;
