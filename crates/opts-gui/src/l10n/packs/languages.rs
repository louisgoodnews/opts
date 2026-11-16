//! The language list and the per-language report of the packs: what each language offers,
//! its statistics and whether it can be selected.

use opts_l10n::builtin;
use opts_l10n::check::{self, Reference};
use opts_l10n::pack::Direction;

use crate::l10n::pseudo::PSEUDO_TAG;
use crate::l10n::{LanguageInfo, LanguageReport, Localization};

impl Localization {
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
