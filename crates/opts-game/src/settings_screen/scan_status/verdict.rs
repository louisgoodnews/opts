//! The verdict of the scan chip: the rules, the change check of the OMSI folder, and the inputs
//! gathered from the background facts.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::content_facts::{ContentFactsStore, RootState};
use crate::settings::GameSettings;

use super::{ScanInputs, ScanLevel, ScanReason, ScanStatus};

/// The rules in the table above.
#[must_use]
pub fn assess(i: &ScanInputs) -> ScanStatus {
    use ScanLevel::{Good, Needed, Recommended};
    use ScanReason as R;
    let (level, reason) = if i.root != RootState::Present {
        (Needed, R::FolderMissing)
    } else if i.failed {
        (Needed, R::Failed)
    } else if i.last_scan.is_none() {
        (
            Needed,
            if i.running {
                R::Running
            } else {
                R::NeverScanned
            },
        )
    } else if i.entities <= 0 {
        (Needed, R::Empty)
    } else if i.running {
        (Recommended, R::Running)
    } else if i.folder_changed {
        (Recommended, R::FolderChanged)
    } else if i
        .last_scan
        .is_some_and(|t| i.now - t > i64::from(i.interval_days.max(1)) * 86_400)
    {
        (Recommended, R::Older)
    } else {
        (Good, R::Recent)
    };
    ScanStatus { level, reason }
}

/// The cheap change check: the OMSI folder or one of its direct sub-folders (`Vehicles`, `maps`,
/// `Sceneryobjects`, ...) has a modification time after `last_scan`. Adding or removing a map or
/// vehicle changes the folder that holds it; no file below them is read.
#[must_use]
pub fn folder_changed_since(root: &Path, last_scan: i64) -> bool {
    let newer = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .is_some_and(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX) > last_scan)
    };
    if newer(root) {
        return true;
    }
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.path().is_dir() && newer(&e.path()))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Builds the inputs from the background facts and what is cheap to read here (the settings and
/// the service's status, which is a mutex around an enum). `None` while the facts are not there
/// yet: the chip then shows its "Checking..." state.
pub fn gather(
    settings: &GameSettings,
    service: Option<&CatalogueService>,
    store: &ContentFactsStore,
) -> Option<ScanInputs> {
    let light = store.light()?;
    let status = service.map(CatalogueService::status);
    Some(ScanInputs {
        now: now_secs(),
        last_scan: light.last_scan,
        entities: light.entities,
        interval_days: settings.discovery_interval_days,
        root: light.root,
        folder_changed: light.folder_changed,
        failed: matches!(status, Some(DiscoveryStatus::Failed(_))),
        running: matches!(status, Some(DiscoveryStatus::Running(_))),
    })
}
