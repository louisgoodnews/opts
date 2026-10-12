//! Path lookups backed by the SQLite catalogue, opened **read-only** (Q-SB-08). They are the
//! same lookups the game hands to its loader (`opts_game::catalogue_service`), so objects
//! resolve their files the way they do in the game.

use std::path::Path;
use std::sync::Arc;

use opts_bevy::install::{NameLookup, PathLookup, RelocateLookup, TwinLookup};
use opts_bevy::OmsiInstall;
use opts_catalogue::Catalogue;

/// Opens the catalogue read-only; `None` (with a warning) when it is absent or unreadable.
#[must_use]
pub fn open(path: Option<&Path>) -> Option<Arc<Catalogue>> {
    let path = path?;
    match Catalogue::open_read_only(path) {
        Ok(c) => Some(Arc::new(c)),
        Err(e) => {
            tracing::warn!(path = %path.display(), "catalogue not usable, resolving paths on disk: {e}");
            None
        }
    }
}

/// An installation handle with the catalogue's lookups.
#[must_use]
pub fn install(root: &Path, catalogue: Option<&Arc<Catalogue>>) -> OmsiInstall {
    let install = OmsiInstall::new(root);
    match catalogue {
        Some(c) => install
            .with_lookup(path_lookup(c))
            .with_name_lookup(name_lookup(c))
            .with_twin_lookup(twin_lookup(c))
            .with_relocate_lookup(relocate_lookup(c)),
        None => install,
    }
}

/// Relative path → file or folder.
#[must_use]
pub fn path_lookup(cat: &Arc<Catalogue>) -> PathLookup {
    let cat = cat.clone();
    Arc::new(move |rel: &str| {
        cat.find_file(rel)
            .ok()
            .flatten()
            .or_else(|| cat.find_dir(rel).ok().flatten())
    })
}

/// Same-named file near a folder.
#[must_use]
pub fn name_lookup(cat: &Arc<Catalogue>) -> NameLookup {
    let cat = cat.clone();
    Arc::new(move |name: &str, near: &str, alt: bool| {
        cat.find_by_name(name, near, alt).ok().flatten()
    })
}

/// Files installed under another folder name than their reference says.
#[must_use]
pub fn relocate_lookup(cat: &Arc<Catalogue>) -> RelocateLookup {
    let cat = cat.clone();
    Arc::new(move |rel: &str| cat.find_relocated(rel).ok().flatten())
}

/// All files with a name (plain twins of scrambled meshes).
#[must_use]
pub fn twin_lookup(cat: &Arc<Catalogue>) -> TwinLookup {
    let cat = cat.clone();
    Arc::new(move |name: &str| cat.find_all_by_name(name, 64).unwrap_or_default())
}
