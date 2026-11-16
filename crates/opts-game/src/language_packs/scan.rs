//! Scanning the language folder: the packs are read into the localisation at start-up and on
//! demand, what is wrong is logged, and a locale without a pack falls back to English.


use bevy::prelude::*;
use opts_gui::l10n::Localization;
use opts_l10n::check::Severity;


use super::packs_dir;

/// What a scan found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSummary {
    /// Packs that can be offered.
    pub usable: usize,
    /// Folders that cannot be used.
    pub broken: usize,
    /// Errors over all packs.
    pub errors: usize,
}

/// Scans the pack folder into `loc`, logs what is wrong, and returns the counts. A locale that
/// has no table after the scan (its pack was removed) falls back to English.
pub fn scan(loc: &mut Localization) -> ScanSummary {
    let Some(dir) = packs_dir() else {
        loc.clear_packs();
        return ScanSummary::default();
    };
    scan_dir(loc, &dir)
}

/// [`scan`] for an explicit folder.
pub fn scan_dir(loc: &mut Localization, dir: &std::path::Path) -> ScanSummary {
    let usable = loc.load_packs(dir);
    let mut summary = ScanSummary {
        usable,
        ..ScanSummary::default()
    };
    for (folder, issues) in loc.pack_problems() {
        summary.broken += 1;
        for i in &issues {
            warn!("language pack `{folder}`: {i}");
        }
    }
    for l in loc.languages().iter().filter(|l| l.from_pack) {
        summary.errors += l.errors;
        if let Some(report) = loc.language_report(&l.code) {
            for i in &report.issues {
                match i.severity {
                    Severity::Error => warn!("language pack `{}`: {i}", l.code),
                    Severity::Warning => debug!("language pack `{}`: {i}", l.code),
                }
            }
        }
    }
    let current = loc.locale().to_string();
    let known = |tag: &str| loc.available().contains(&tag);
    let language = current.split('-').next().unwrap_or(&current).to_string();
    if !known(&current) && !known(&language) {
        info!("language `{current}` is no longer installed; showing English");
        loc.set_locale("en");
    }
    summary
}

/// Start-up: reads the pack folder after every plugin has added its English messages.
pub fn load_startup(mut loc: ResMut<Localization>) {
    let s = scan(&mut loc);
    if s.usable + s.broken > 0 {
        info!(
            usable = s.usable,
            broken = s.broken,
            errors = s.errors,
            "language packs scanned"
        );
    }
}
