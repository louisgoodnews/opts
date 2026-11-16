//! Listing, finding, renaming and deleting saves.

use super::*;

impl SaveStore {
    /// Deletes save `id` with its backup and any leftover temporary file.
    ///
    /// # Errors
    /// No such save, or the file system.
    pub fn delete(&self, id: &str) -> Result<(), SaveError> {
        Self::check(id)?;
        let mut found = false;
        for path in [
            self.main_path(id),
            self.with_suffix(id, "bak"),
            self.with_suffix(id, "tmp"),
        ] {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| SaveError::Io(format!("{}: {e}", path.display())))?;
                found = true;
            }
        }
        if found {
            Ok(())
        } else {
            Err(SaveError::NotFound(id.to_owned()))
        }
    }

    /// Renames save `id` (its display name; the file keeps its id).
    ///
    /// # Errors
    /// No such or unreadable save, an empty name, or the file system.
    pub fn rename(&self, id: &str, new_name: &str) -> Result<(), SaveError> {
        let name = super::super::container::clean_name(new_name)?;
        let mut loaded = self.read(id)?;
        loaded.file.meta.name = name;
        self.write(id, &loaded.file)
    }

    /// Every save in the folder, most recently saved first; damaged ones are listed with their
    /// error (sorted last). Leftover `.tmp` files are ignored.
    #[must_use]
    pub fn list(&self) -> Vec<SaveEntry> {
        let mut ids: Vec<String> = Vec::new();
        for entry in fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name
                .strip_suffix(&format!(".{EXT}"))
                .or_else(|| name.strip_suffix(&format!(".{EXT}.bak")));
            if let Some(id) = stem.filter(|s| valid_id(s)) {
                if !ids.iter().any(|i| i == id) {
                    ids.push(id.to_owned());
                }
            }
        }
        let mut entries: Vec<SaveEntry> = ids
            .into_iter()
            .map(|id| SaveEntry {
                state: self.read(&id),
                id,
            })
            .collect();
        entries.sort_by(|a, b| {
            let key = |e: &SaveEntry| e.meta().map(|m| m.saved_unix);
            key(b).cmp(&key(a)).then_with(|| a.id.cmp(&b.id))
        });
        entries
    }

    /// The id of the save called `name` (exact, then case-insensitive), or whose id is `name`.
    #[must_use]
    pub fn find(&self, name_or_id: &str) -> Option<String> {
        let entries = self.list();
        entries
            .iter()
            .find(|e| e.id == name_or_id)
            .or_else(|| {
                entries
                    .iter()
                    .find(|e| e.meta().is_some_and(|m| m.name == name_or_id))
            })
            .or_else(|| {
                entries.iter().find(|e| {
                    e.meta()
                        .is_some_and(|m| m.name.eq_ignore_ascii_case(name_or_id))
                })
            })
            .map(|e| e.id.clone())
    }
}
