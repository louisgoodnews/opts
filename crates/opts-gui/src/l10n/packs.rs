//! Language packs in the localisation: scanning the user folder, the language list and the
//! per-language report. The pure parts (format, checks) live in `opts_core::l10n`.

use std::collections::HashMap;
use std::path::Path;

use opts_core::l10n::builtin;
use opts_core::l10n::check::{self, Issue, Reference};
use opts_core::l10n::fluent::Table;
use opts_core::l10n::pack::{self, Direction};

use super::pseudo::PSEUDO_TAG;
use super::{LanguageInfo, LanguageReport, LoadedPack, Localization};

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

    fn info_for(&self, reference: &Reference, code: &str) -> Option<LanguageInfo> {
        let loaded = self
            .packs
            .iter()
            .find(|p| p.pack.meta.as_ref().is_some_and(|m| m.code == code));
        let builtin_meta = builtin::meta_of(code);
        let meta = builtin_meta
            .clone()
            .or_else(|| loaded.and_then(|p| p.pack.meta.clone()))?;
        let table = self.effective(code);
        let stats = check::stats(reference, &table, code);
        let (errors, warnings, outdated, missing_fonts, dir) =
            loaded.map_or((0, 0, false, Vec::new(), None), |p| {
                (
                    p.pack.errors(),
                    p.pack.warnings(),
                    p.pack.outdated,
                    p.pack
                        .missing_fonts
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                    Some(p.pack.dir.clone()),
                )
            });
        let direction = loaded
            .and_then(|p| p.pack.meta.as_ref().map(|m| m.direction))
            .filter(|_| builtin_meta.is_none())
            .unwrap_or(meta.direction);
        Some(LanguageInfo {
            code: code.to_string(),
            name: meta.name,
            native_name: meta.native_name,
            direction,
            builtin: builtin_meta.is_some(),
            from_pack: loaded.is_some(),
            authors: loaded
                .and_then(|p| p.pack.meta.as_ref().map(|m| m.authors.clone()))
                .filter(|_| builtin_meta.is_none())
                .unwrap_or(meta.authors),
            version: meta.version,
            total: stats.total,
            translated: stats.translated,
            copied: stats.copied,
            missing: stats.missing.len(),
            errors,
            warnings,
            outdated,
            missing_fonts,
            dir,
            selectable: direction == Direction::Ltr,
        })
    }

    /// The languages the interface can be shown in: `en`, `de`, then the usable packs by tag.
    #[must_use]
    pub fn languages(&self) -> Vec<LanguageInfo> {
        let reference = self.reference();
        let mut codes: Vec<String> = builtin::CODES.iter().map(|c| (*c).to_string()).collect();
        let mut extra: Vec<String> = self
            .packs
            .iter()
            .filter_map(|p| p.pack.meta.as_ref().map(|m| m.code.clone()))
            .filter(|c| !codes.contains(c))
            .collect();
        extra.sort();
        extra.dedup();
        codes.extend(extra);
        codes
            .iter()
            .filter_map(|c| self.info_for(&reference, c))
            .collect()
    }

    /// The report of one language; `None` for a tag that is neither built in nor a usable pack.
    #[must_use]
    pub fn language_report(&self, code: &str) -> Option<LanguageReport> {
        let reference = self.reference();
        let info = self.info_for(&reference, code)?;
        let stats = check::stats(&reference, &self.effective(code), code);
        let copied_keys = reference
            .keys()
            .iter()
            .filter(|k| {
                let t = self.effective(code);
                t.messages.get(*k).is_some_and(|p| {
                    code != "en"
                        && reference.table.messages.get(*k) == Some(p)
                        && !stats.missing.contains(k)
                        && check::visible_text(p).split_whitespace().count() >= 2
                })
            })
            .cloned()
            .collect();
        let issues = self
            .packs
            .iter()
            .find(|p| p.pack.meta.as_ref().is_some_and(|m| m.code == code))
            .map(|p| p.pack.issues.clone())
            .unwrap_or_default();
        Some(LanguageReport {
            info,
            missing_keys: stats.missing,
            copied_keys,
            issues,
        })
    }

    /// `true` when `code` has a table (built in, pack or the test pseudo-locale) and may be
    /// selected: right-to-left packs may not yet.
    #[must_use]
    pub fn can_select(&self, code: &str) -> bool {
        if code == PSEUDO_TAG {
            return true;
        }
        self.languages()
            .iter()
            .any(|l| l.code == code && l.selectable)
    }
}
