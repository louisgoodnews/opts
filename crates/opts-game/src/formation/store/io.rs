//! Writing, reading and deleting formation files, and listing the saved ones.

use super::*;

impl FormationStore {
    /// Writes `formation` as `id` (replacing that file), atomically.
    ///
    /// # Errors
    /// [`StoreError`] for a bad id or a file system failure.
    pub fn write(&self, id: &str, formation: &TrainFormation) -> Result<(), StoreError> {
        Self::check(id)?;
        let text = encode(formation)?;
        let io = |e: std::io::Error| StoreError::Io(format!("{}: {e}", self.dir.display()));
        fs::create_dir_all(&self.dir).map_err(io)?;
        let tmp = self.dir.join(format!("{id}.{EXT}.tmp"));
        fs::write(&tmp, text).map_err(io)?;
        fs::rename(&tmp, self.path(id)).map_err(io)
    }

    /// Saves `formation` under a new id made from its name, and returns the id.
    ///
    /// # Errors
    /// [`StoreError`] on a file system failure.
    pub fn save_new(&self, formation: &TrainFormation) -> Result<String, StoreError> {
        let id = self.unique_id(&formation.name);
        self.write(&id, formation)?;
        Ok(id)
    }

    /// Reads formation `id`.
    ///
    /// # Errors
    /// [`StoreError`] when it does not exist or cannot be read.
    pub fn read(&self, id: &str) -> Result<TrainFormation, StoreError> {
        Self::check(id)?;
        Self::read_file(&self.path(id), id)
    }

    fn read_file(path: &Path, id: &str) -> Result<TrainFormation, StoreError> {
        let meta = fs::metadata(path).map_err(|_| StoreError::NotFound(id.to_owned()))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err(StoreError::Corrupt(format!(
                "{} is {} bytes (limit {MAX_FILE_BYTES})",
                path.display(),
                meta.len()
            )));
        }
        let bytes =
            fs::read(path).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| StoreError::Corrupt("not UTF-8 text".to_owned()))?;
        decode(&text)
    }

    /// Deletes formation `id`.
    ///
    /// # Errors
    /// [`StoreError`] when it does not exist or cannot be removed.
    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        Self::check(id)?;
        fs::remove_file(self.path(id)).map_err(|_| StoreError::NotFound(id.to_owned()))
    }

    /// Every file of the store, newest first (files that cannot be read are listed with their
    /// error).
    #[must_use]
    pub fn list(&self) -> Vec<SavedFormation> {
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<SavedFormation> = rd
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                if path.extension().is_none_or(|x| x != EXT) {
                    return None;
                }
                let id = path.file_stem()?.to_string_lossy().into_owned();
                if !valid_id(&id) {
                    return None;
                }
                Some(SavedFormation {
                    state: Self::read_file(&path, &id),
                    modified: e.metadata().ok().and_then(|m| m.modified().ok()),
                    id,
                })
            })
            .collect();
        out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));
        out
    }
}
