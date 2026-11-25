//! Mesh notes: what the loader learned about a mesh file, for the records placed with it.

use std::path::Path;

use super::*;

/// Stores what the loader learned about a mesh file; later records pick it up with [`note_for`].
pub fn note_mesh(path: &Path, mut note: MeshNote) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            note.file = s.relative(path);
            s.notes.insert(store::norm(path), note);
        }
    }
}

/// Whether a note is stored for the file.
#[must_use]
pub fn has_note(path: &Path) -> bool {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.notes.contains_key(&store::norm(path))))
        .unwrap_or(false)
}

/// The note stored for a mesh file, or a bare `plain` note when the file was served from the
/// loader's caches without a fresh load.
#[must_use]
pub fn note_for(path: &Path) -> MeshNote {
    let Ok(state) = STATE.lock() else {
        return MeshNote::default();
    };
    let Some(s) = state.as_ref() else {
        return MeshNote::default();
    };
    s.notes
        .get(&store::norm(path))
        .cloned()
        .unwrap_or_else(|| MeshNote {
            file: s.relative(path),
            stage: "unknown".into(),
            ..MeshNote::default()
        })
}

/// A path relative to the OMSI folder (privacy: never an absolute path).
#[must_use]
pub fn relative(path: &Path) -> String {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.relative(path)))
        .unwrap_or_else(|| store::file_name_of(path))
}
