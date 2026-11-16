//! Reading and writing save files: the decoding of a file, the checked read, the atomic write.

use super::*;

impl SaveStore {
    pub(super) fn read_file(path: &Path) -> Result<SaveFile, SaveError> {
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
    pub(super) fn write_text(&self, id: &str, text: &str) -> Result<(), SaveError> {
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

}
