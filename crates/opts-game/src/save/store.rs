//! Files of the save list: where they live, atomic writes, the rotating backup, recovery.
//!
//! `<dir>/<id>.save.json` is the save, `<id>.save.json.bak` the previous version, and
//! `<id>.save.json.tmp` exists only while a write is in progress. A write never touches the main
//! file until the new text is complete on disk:
//!
//! 1. write the text to `.tmp` and `fsync` it;
//! 2. move the old main file to `.bak` (replacing the older backup);
//! 3. rename `.tmp` to the main file.
//!
//! A crash before step 3 leaves either the old main file or (between 2 and 3) only the backup,
//! which [`SaveStore::read`] falls back to.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use super::container::{slug, valid_id, SaveError, SaveFile, SaveMeta, MAX_FILE_BYTES};

const EXT: &str = "save.json";

/// A folder of saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveStore {
    dir: PathBuf,
}

/// A save read from the folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The save.
    pub file: SaveFile,
    /// The main file was missing or damaged and the backup was used.
    pub from_backup: bool,
}

/// One line of the save list.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveEntry {
    /// File stem; what the control protocol and the UI refer to.
    pub id: String,
    /// The save, or why it cannot be read.
    pub state: Result<Loaded, SaveError>,
}

impl SaveEntry {
    /// The metadata when the file could be read.
    #[must_use]
    pub fn meta(&self) -> Option<&SaveMeta> {
        self.state.as_ref().ok().map(|l| &l.file.meta)
    }
}

impl SaveStore {
    /// A store in `dir` (created on the first write).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The folder of the user's saves: `<user data dir>/saves`.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("saves"))
    }

    /// The user's store, if a data directory is known.
    #[must_use]
    pub fn user() -> Option<Self> {
        Self::default_dir().map(Self::new)
    }

    /// The folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn main_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}"))
    }

    fn with_suffix(&self, id: &str, suffix: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}.{suffix}"))
    }

    fn check(id: &str) -> Result<(), SaveError> {
        if valid_id(id) {
            Ok(())
        } else {
            Err(SaveError::InvalidName(format!("`{id}` is not a save id")))
        }
    }

    /// An id for a new save called `name` that no file uses yet.
    #[must_use]
    pub fn unique_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.main_path(&id).exists() || self.with_suffix(&id, "bak").exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    fn read_file(path: &Path) -> Result<SaveFile, SaveError> {
        let meta =
            fs::metadata(path).map_err(|e| SaveError::Io(format!("{}: {e}", path.display())))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(SaveError::TooLarge(format!(
                "{} is {} bytes (limit {MAX_FILE_BYTES})",
                path.display(),
                meta.len()
            )));
        }
        let bytes =
            fs::read(path).map_err(|e| SaveError::Io(format!("{}: {e}", path.display())))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| SaveError::Corrupt("not UTF-8 text".to_owned()))?;
        SaveFile::decode(&text)
    }

    /// Reads save `id`. A damaged or missing main file falls back to the backup.
    ///
    /// # Errors
    /// The main file's error when there is no usable backup either (a file from a newer build is
    /// reported as such and never replaced by the backup).
    pub fn read(&self, id: &str) -> Result<Loaded, SaveError> {
        Self::check(id)?;
        let main = self.main_path(id);
        let bak = self.with_suffix(id, "bak");
        let first = if main.exists() {
            match Self::read_file(&main) {
                Ok(file) => {
                    return Ok(Loaded {
                        file,
                        from_backup: false,
                    })
                }
                Err(e @ SaveError::NewerFormat { .. }) => return Err(e),
                Err(e) => e,
            }
        } else if bak.exists() {
            SaveError::NotFound(id.to_owned())
        } else {
            return Err(SaveError::NotFound(id.to_owned()));
        };
        match Self::read_file(&bak) {
            Ok(file) => {
                tracing::warn!(id, "the save file is unusable ({first}); using its backup");
                Ok(Loaded {
                    file,
                    from_backup: true,
                })
            }
            Err(_) => Err(first),
        }
    }

    /// Writes `file` as save `id` (atomically, keeping the previous version as the backup).
    ///
    /// # Errors
    /// Encoding limits or the file system.
    pub fn write(&self, id: &str, file: &SaveFile) -> Result<(), SaveError> {
        Self::check(id)?;
        let text = file.encode()?;
        self.write_text(id, &text)
    }

    /// The write steps for already encoded text (tests break them apart through [`Self::write_tmp`]).
    fn write_text(&self, id: &str, text: &str) -> Result<(), SaveError> {
        self.write_tmp(id, text.as_bytes())?;
        self.commit(id)
    }

    /// Step 1: the complete new text in `<id>.save.json.tmp`, flushed to disk.
    pub(crate) fn write_tmp(&self, id: &str, bytes: &[u8]) -> Result<(), SaveError> {
        let io = |e: std::io::Error| SaveError::Io(e.to_string());
        fs::create_dir_all(&self.dir).map_err(io)?;
        let mut tmp = fs::File::create(self.with_suffix(id, "tmp")).map_err(io)?;
        tmp.write_all(bytes).map_err(io)?;
        tmp.sync_all().map_err(io)
    }

    /// Steps 2 and 3: old main file to the backup, temporary file to the main file.
    pub(crate) fn commit(&self, id: &str) -> Result<(), SaveError> {
        let io = |e: std::io::Error| SaveError::Io(e.to_string());
        let main = self.main_path(id);
        if main.exists() {
            fs::rename(&main, self.with_suffix(id, "bak")).map_err(io)?;
        }
        fs::rename(self.with_suffix(id, "tmp"), &main).map_err(io)?;
        if let Ok(dir) = fs::File::open(&self.dir) {
            let _ = dir.sync_all();
        }
        Ok(())
    }

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
        let name = super::container::clean_name(new_name)?;
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

#[cfg(test)]
mod tests;
