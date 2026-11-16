//! The real facts provider: the catalogue file (read only), the OMSI folder and the texture
//! cache folder.

use super::{FactsProvider, FactsRequest, HeavyFacts, LightFacts, RootState};

/// The real provider: the catalogue file, the OMSI folder and the texture cache folder.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiskFacts;

impl FactsProvider for DiskFacts {
    fn light(&self, req: &FactsRequest) -> LightFacts {
        let root = req.root.clone().or_else(crate::omsi_root::known_omsi_root);
        let root_state = match &root {
            None => RootState::Unset,
            Some(r) if r.is_dir() => RootState::Present,
            Some(_) => RootState::Missing,
        };
        let (last_scan, entities) = open(req).map_or((None, 0), |c| {
            (c.last_scan().ok().flatten(), c.totals().map_or(0, |t| t.1))
        });
        let folder_changed = matches!((&root, last_scan), (Some(r), Some(t)) if r.is_dir()
            && crate::settings_screen::scan_status::folder_changed_since(r, t));
        LightFacts {
            last_scan,
            entities,
            root: root_state,
            folder_changed,
        }
    }

    fn theme_files(&self) -> Vec<String> {
        crate::theme_files::discover_theme_files()
    }

    fn heavy(&self, req: &FactsRequest) -> HeavyFacts {
        let kinds = open(req)
            .and_then(|c| c.kind_counts().ok())
            .unwrap_or_default();
        HeavyFacts { kinds }
    }
}

/// A read-only handle: no file is created, no schema migrated, nothing locks the scan's writes.
fn open(req: &FactsRequest) -> Option<opts_catalogue::Catalogue> {
    req.db_path
        .as_deref()
        .filter(|p| p.exists())
        .and_then(|p| opts_catalogue::Catalogue::open_read_only(p).ok())
}
