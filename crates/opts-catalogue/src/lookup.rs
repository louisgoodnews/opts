//! Finding files and directories in the scanned install, with relocation.

use super::relocation::{best_candidate, normalize, pick_relocated};
use super::*;

impl Catalogue {
    /// Resolves an OMSI-relative path (any case, `\` or `/`) to an absolute path, using the
    /// cached index instead of touching the filesystem.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_file(&self, rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(rel);
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Option<String> = self.with(|c| {
            c.prepare_cached("SELECT rel_path FROM files WHERE rel_path = ?1 COLLATE NOCASE")?
                .query_row([&rel], |r| r.get(0))
                .optional()
        })?;
        Ok(found.map(|p| root.join(p)))
    }

    /// All files with the given file name (any case), up to `limit`, as absolute paths.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_all_by_name(&self, name: &str, limit: usize) -> Result<Vec<PathBuf>> {
        let base = normalize(name);
        let base = base.rsplit('/').next().unwrap_or("").to_string();
        let Some(root) = self.root()? else {
            return Ok(Vec::new());
        };
        if base.is_empty() {
            return Ok(Vec::new());
        }
        let found: Vec<String> = self.with(|c| {
            let mut st = c.prepare_cached(
                "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT ?2",
            )?;
            let rows = st.query_map(params![base, limit as i64], |r| r.get::<_, String>(0))?;
            rows.collect()
        })?;
        Ok(found.into_iter().map(|p| root.join(p)).collect())
    }

    /// Finds a file by name alone (any case), preferring the one closest to `near_rel_dir`.
    ///
    /// Used to repair references whose path is wrong but whose file exists elsewhere. If
    /// `with_alternative_extensions` is set and the exact name is absent, the same stem with the
    /// other image extensions is tried.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_by_name(
        &self,
        name: &str,
        near_rel_dir: &str,
        with_alternative_extensions: bool,
    ) -> Result<Option<PathBuf>> {
        let base = normalize(name);
        let base = base.rsplit('/').next().unwrap_or("").to_string();
        if base.is_empty() {
            return Ok(None);
        }
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let stem = base
            .rsplit_once('.')
            .map_or(base.as_str(), |(s, _)| s)
            .to_string();
        let mut names = vec![base.clone()];
        if with_alternative_extensions {
            names.extend(
                ["dds", "bmp", "tga", "png", "jpg"]
                    .iter()
                    .map(|e| format!("{stem}.{e}")),
            );
        }
        let near = normalize(near_rel_dir);
        for n in names {
            let found: Vec<String> = self.with(|c| {
                let mut st = c.prepare_cached(
                    "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT 64",
                )?;
                let rows = st.query_map([&n], |r| r.get(0))?;
                rows.collect()
            })?;
            if let Some(best) = best_candidate(&found, &near) {
                return Ok(Some(root.join(best)));
            }
        }
        Ok(None)
    }

    /// Finds the real location of a referenced file that does not exist at its stated path
    /// (content packs are often installed under other folder names than the map was made with).
    ///
    /// Candidates are the indexed files with the same name. A single candidate is taken; among
    /// several, the one whose folder names match the reference's best (equal trailing folders,
    /// then shared name tokens) wins, but only when it beats the runner-up clearly.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_relocated(&self, referenced_rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(referenced_rel);
        let Some((dir, name)) = rel.rsplit_once('/').or(Some(("", rel.as_str()))) else {
            return Ok(None);
        };
        if name.is_empty() {
            return Ok(None);
        }
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Vec<String> = self.with(|c| {
            let mut st = c.prepare_cached(
                "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT 256",
            )?;
            let rows = st.query_map([name], |r| r.get(0))?;
            rows.collect()
        })?;
        Ok(pick_relocated(&found, dir).map(|c| root.join(c)))
    }

    /// Resolves an OMSI-relative directory path, like [`Catalogue::find_file`].
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_dir(&self, rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(rel);
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Option<String> = self.with(|c| {
            c.prepare_cached("SELECT rel_path FROM directories WHERE rel_path = ?1 COLLATE NOCASE")?
                .query_row([&rel], |r| r.get(0))
                .optional()
        })?;
        Ok(found.map(|p| root.join(p)))
    }
}
