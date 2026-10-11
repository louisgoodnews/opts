//! Case-insensitive resolution of OMSI content paths inside an installation directory.
//!
//! OMSI content is authored on Windows: references such as `Sceneryobjects\Foo\bar.sco` must
//! resolve on case-sensitive filesystems. [`OmsiInstall`] is the single path-resolution service
//! shared by the model loader, scenery/spline spawning and the game's session loader.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Optional cache lookup: OMSI-root-relative path → absolute path (e.g. a SQLite catalogue).
pub type PathLookup = Arc<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;

/// Name-only lookup `(file name, near OMSI-relative directory, try other image extensions)`
/// → absolute path; repairs references whose path is wrong but whose file exists elsewhere.
pub type NameLookup = Arc<dyn Fn(&str, &str, bool) -> Option<PathBuf> + Send + Sync>;

/// Reference path (OMSI-relative) → real location of a file installed under other folder names.
pub type RelocateLookup = Arc<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;

/// All indexed files with a given file name (any case), used to find plain twins of
/// scrambled meshes.
pub type TwinLookup = Arc<dyn Fn(&str) -> Vec<PathBuf> + Send + Sync>;

use bevy::ecs::resource::Resource;

/// Handle to an OMSI installation directory with case-insensitive file lookup.
#[derive(Resource)]
pub struct OmsiInstall {
    root: PathBuf,
    lookup: Option<PathLookup>,
    name_lookup: Option<NameLookup>,
    twin_lookup: Option<TwinLookup>,
    relocate_lookup: Option<RelocateLookup>,
    dir_cache: Mutex<HashMap<PathBuf, Arc<HashMap<String, PathBuf>>>>,
}

impl std::fmt::Debug for OmsiInstall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OmsiInstall")
            .field("root", &self.root)
            .field("indexed", &self.lookup.is_some())
            .finish()
    }
}

impl OmsiInstall {
    /// Creates a handle for the installation at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            lookup: None,
            name_lookup: None,
            twin_lookup: None,
            relocate_lookup: None,
            dir_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Attaches a cached path index consulted before the filesystem is touched.
    #[must_use]
    pub fn with_lookup(mut self, lookup: PathLookup) -> Self {
        self.lookup = Some(lookup);
        self
    }

    /// Attaches a by-name index used to repair wrong references.
    #[must_use]
    pub fn with_name_lookup(mut self, lookup: NameLookup) -> Self {
        self.name_lookup = Some(lookup);
        self
    }

    /// Attaches the all-files-by-name index.
    #[must_use]
    pub fn with_twin_lookup(mut self, lookup: TwinLookup) -> Self {
        self.twin_lookup = Some(lookup);
        self
    }

    /// Attaches the lookup that finds relocated files.
    #[must_use]
    pub fn with_relocate_lookup(mut self, lookup: RelocateLookup) -> Self {
        self.relocate_lookup = Some(lookup);
        self
    }

    /// Real location of `rel` when the file is not at that path but installed elsewhere under the
    /// same name (see `Catalogue::find_relocated`).
    #[must_use]
    pub fn find_relocated(&self, rel: &str) -> Option<PathBuf> {
        self.relocate_lookup.as_ref().and_then(|l| l(rel))
    }

    /// All indexed files with this file name (empty without an index).
    #[must_use]
    pub fn files_named(&self, name: &str) -> Vec<PathBuf> {
        self.twin_lookup.as_ref().map_or_else(Vec::new, |l| l(name))
    }

    /// Finds a file by name alone, closest to `near_dir`, using the attached index.
    #[must_use]
    pub fn find_by_name(
        &self,
        name: &str,
        near_dir: &Path,
        alt_extensions: bool,
    ) -> Option<PathBuf> {
        let lookup = self.name_lookup.as_ref()?;
        // The directory may be spelled through a symlink of the root (or the other way round).
        let relative = near_dir
            .strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .or_else(|_| {
                let (root, near) = (self.root.canonicalize()?, near_dir.canonicalize()?);
                near.strip_prefix(&root)
                    .map(Path::to_path_buf)
                    .map_err(std::io::Error::other)
            });
        let near = relative.ok()?.to_string_lossy().into_owned();
        lookup(name, &near, alt_extensions)
    }

    /// Installation root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Derives the installation root from a map folder (`<root>/maps/<name>`).
    #[must_use]
    pub fn root_from_map_dir(map_dir: &Path) -> Option<PathBuf> {
        let maps = map_dir.parent()?;
        if !maps
            .file_name()?
            .to_string_lossy()
            .eq_ignore_ascii_case("maps")
        {
            return None;
        }
        maps.parent().map(Path::to_path_buf)
    }

    /// Resolves `rel` (backslash or slash separated) against the installation root.
    #[must_use]
    pub fn find_file(&self, rel: &str) -> Option<PathBuf> {
        self.find_in(&self.root, rel)
    }

    /// Resolves `rel` against an arbitrary base directory, ignoring case.
    #[must_use]
    pub fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        if let (Some(lookup), Ok(prefix)) = (&self.lookup, base.strip_prefix(&self.root)) {
            let joined = format!("{}/{}", prefix.to_string_lossy(), rel);
            if let Some(found) = lookup(&joined) {
                return Some(found);
            }
        }
        let mut cur = base.to_path_buf();
        for comp in rel.split(['\\', '/']) {
            let comp = comp.trim();
            if comp.is_empty() || comp == "." {
                continue;
            }
            if comp == ".." {
                cur.pop();
                continue;
            }
            let direct = cur.join(comp);
            if direct.exists() {
                cur = direct;
                continue;
            }
            let listing = self.listing(&cur)?;
            cur = listing.get(&comp.to_lowercase())?.clone();
        }
        cur.exists().then_some(cur)
    }

    fn listing(&self, dir: &Path) -> Option<Arc<HashMap<String, PathBuf>>> {
        let mut cache = self.dir_cache.lock().ok()?;
        if let Some(found) = cache.get(dir) {
            return Some(found.clone());
        }
        let mut map = HashMap::new();
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            map.insert(
                entry.file_name().to_string_lossy().to_lowercase(),
                entry.path(),
            );
        }
        let map = Arc::new(map);
        cache.insert(dir.to_path_buf(), map.clone());
        Some(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_lookup_ignores_case_and_separators() {
        let dir = std::env::temp_dir().join(format!("opts_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Sceneryobjects/Foo")).unwrap();
        std::fs::write(dir.join("Sceneryobjects/Foo/Bar.sco"), b"x").unwrap();
        let install = OmsiInstall::new(&dir);
        assert!(install.find_file("sceneryobjects\\foo\\bar.SCO").is_some());
        assert!(install
            .find_file("sceneryobjects\\foo\\missing.sco")
            .is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
