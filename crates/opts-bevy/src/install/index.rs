//! Resolving content paths inside the installation: relative lookups, folder case and the listing index.

use super::*;

impl OmsiInstall {
    /// Resolves `rel` (backslash or slash separated) against the installation root.
    #[must_use]
    pub fn find_file(&self, rel: &str) -> Option<PathBuf> {
        self.find_in(&self.root, rel)
    }

    /// Resolves `rel` against an arbitrary base directory, ignoring case.
    #[must_use]
    pub fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        if let (Some(lookup), Some(prefix)) = (&self.lookup, self.relative_to_root(base)) {
            let joined = format!("{}/{}", prefix.to_string_lossy(), rel);
            if let Some(found) = lookup(&joined) {
                return Some(found);
            }
        }
        let mut cur = self.existing_dir_ignoring_case(base)?;
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

    /// `base` relative to the installation root, whether it is spelled through the root as given
    /// or through the root with symlinks resolved (the spelling catalogue paths use).
    pub(super) fn relative_to_root(&self, base: &Path) -> Option<PathBuf> {
        base.strip_prefix(&self.root)
            .ok()
            .or_else(|| base.strip_prefix(self.canonical_root.as_ref()?).ok())
            .map(Path::to_path_buf)
    }

    /// `base` itself when it exists, otherwise the existing directory that equals it up to the
    /// case of its folder names (authors write `texture`, the folder is `Texture`).
    pub(super) fn existing_dir_ignoring_case(&self, base: &Path) -> Option<PathBuf> {
        if base.exists() {
            return Some(base.to_path_buf());
        }
        let mut tail = Vec::new();
        let mut head = base;
        while !head.exists() {
            tail.push(head.file_name()?.to_string_lossy().into_owned());
            head = head.parent()?;
        }
        let mut cur = head.to_path_buf();
        for name in tail.iter().rev() {
            let listing = self.listing(&cur)?;
            cur = listing.get(&name.to_lowercase())?.clone();
        }
        Some(cur)
    }

    pub(super) fn listing(&self, dir: &Path) -> Option<Arc<HashMap<String, PathBuf>>> {
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
