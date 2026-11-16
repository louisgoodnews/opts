//! Persistent store of saved [`BusConfiguration`] entries (`bus_configurations.json`).

use super::*;

/// Why the file could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreProblem {
    /// The file exists but could not be read.
    Unreadable(String),
    /// The file is not valid JSON of the expected shape.
    Corrupt(String),
    /// The file was written by a newer version of the game; it is left as it is.
    Newer(u32),
}

impl fmt::Display for StoreProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(e) => write!(f, "the configurations file cannot be read: {e}"),
            Self::Corrupt(e) => write!(f, "the configurations file is damaged: {e}"),
            Self::Newer(v) => write!(
                f,
                "the configurations file has version {v}, this game reads up to {FORMAT_VERSION}"
            ),
        }
    }
}

/// All saved configurations.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BusConfigurations {
    /// File layout version (0 in files without one).
    #[serde(default)]
    pub version: u32,
    /// The configurations, in the order they were first saved.
    #[serde(default)]
    pub configurations: Vec<BusConfiguration>,
}

/// Whether two names are the same ignoring case (also of umlauts) and surrounding spaces.
pub(super) fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// A slug for an id: lower-case letters and digits, other runs become one `-`.
pub(super) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "configuration".to_string()
    } else {
        out
    }
}

impl BusConfigurations {
    /// Where the file is stored (beside the settings database), if a directory is known.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        GameSettings::database_path().map(|p| p.with_file_name(FILE_NAME))
    }

    /// Reads the store at `path`; a missing file is an empty store.
    ///
    /// # Errors
    /// [`StoreProblem`] when the file cannot be read, is damaged or is from a newer version.
    pub fn load_from(path: &Path) -> Result<Self, StoreProblem> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(StoreProblem::Unreadable(e.to_string())),
        };
        let mut store: Self =
            serde_json::from_str(&text).map_err(|e| StoreProblem::Corrupt(e.to_string()))?;
        if store.version > FORMAT_VERSION {
            return Err(StoreProblem::Newer(store.version));
        }
        // Files before the version field (0) have the layout of version 1.
        store.version = FORMAT_VERSION;
        Ok(store)
    }

    /// Writes the store to `path` (through a temporary file, so a crash keeps the old one). A
    /// damaged file that is there is first kept as `<name>.broken`.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if matches!(Self::load_from(path), Err(StoreProblem::Corrupt(_))) {
            let mut broken = path.as_os_str().to_owned();
            broken.push(".broken");
            std::fs::rename(path, broken)?;
        }
        let mut copy = self.clone();
        copy.version = FORMAT_VERSION;
        let json = serde_json::to_string_pretty(&copy).map_err(std::io::Error::other)?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    /// The configuration with this id, else the one with this name (ignoring case).
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&BusConfiguration> {
        self.configurations
            .iter()
            .find(|c| c.id == key)
            .or_else(|| self.configurations.iter().find(|c| same_name(&c.name, key)))
    }

    fn position(&self, key: &str) -> Option<usize> {
        let id = self.find(key)?.id.clone();
        self.configurations.iter().position(|c| c.id == id)
    }

    /// An id for a new configuration called `name`: its slug, numbered when taken.
    fn fresh_id(&self, name: &str) -> String {
        let base = slug(name);
        let taken = |id: &str| self.configurations.iter().any(|c| c.id == id);
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|id| !taken(id))
            .unwrap_or(base)
    }

}

mod mutate;

