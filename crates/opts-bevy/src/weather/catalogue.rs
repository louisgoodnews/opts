//! The weather catalogue: built-in presets, aliases and lookup.

use super::*;
mod lookup;
mod presets;

pub use presets::builtin_preset;

/// The preset ids of versions before real weather, the default first.
pub const BUILTIN_IDS: [&str; 5] = [
    "Clear Summer Day",
    "Overcast Autumn",
    "Rainy Afternoon",
    "Winter Snowfall",
    "Autumn Morning Fog",
];

/// The installed preset that stands in for a built-in id (`None`: no install preset is close
/// enough, the built-in one is used). See the alias table in `docs/src/architecture/weather.md`.
#[must_use]
pub fn alias_of(builtin_id: &str) -> Option<&'static str> {
    match builtin_id {
        "Clear Summer Day" => Some("#CAVOK"),
        "Rainy Afternoon" => Some("Schmuddelwetter"),
        "Winter Snowfall" => Some("Starker Schneefall"),
        "Autumn Morning Fog" => Some("Bodennebel"),
        _ => None,
    }
}

/// The built-in id an unknown or old preset id stands for, by the words it contains (`rain`,
/// `snow`, `fog`, `overcast`); anything else is the clear default.
#[must_use]
pub fn legacy_id(id: &str) -> &'static str {
    let lower = id.to_lowercase();
    BUILTIN_IDS
        .iter()
        .find(|b| b.eq_ignore_ascii_case(id))
        .copied()
        .unwrap_or(if lower.contains("snow") {
            "Winter Snowfall"
        } else if lower.contains("rain") {
            "Rainy Afternoon"
        } else if lower.contains("fog") {
            "Autumn Morning Fog"
        } else if lower.contains("overcast") {
            "Overcast Autumn"
        } else {
            "Clear Summer Day"
        })
}

/// The presets the game can offer: the install's, or the five built-in ones.
///
/// A resource; the game fills it from the install's `Weather` folder when the root is known.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct WeatherCatalogue {
    pub(super) root: Option<PathBuf>,
    pub(super) installed: Vec<PresetFile>,
    pub(super) clouds: CloudTypes,
}

static BUILTIN_CATALOGUE: OnceLock<WeatherCatalogue> = OnceLock::new();

impl WeatherCatalogue {
    /// The catalogue without an install (what a new [`SessionEnvironment`] resolves with).
    ///
    /// [`SessionEnvironment`]: crate::environment::SessionEnvironment
    #[must_use]
    pub fn builtin() -> &'static Self {
        BUILTIN_CATALOGUE.get_or_init(Self::default)
    }

    /// A catalogue of the presets read from `folder`, found below `root`.
    #[must_use]
    pub fn from_folder(root: PathBuf, folder: WeatherFolder) -> Self {
        Self {
            root: Some(root),
            installed: folder.presets,
            clouds: folder.clouds,
        }
    }

    /// Reads `<root>/Weather` (`None` for a root without presets).
    #[must_use]
    pub fn read(root: &std::path::Path) -> Option<Self> {
        let dir = crate::install::OmsiInstall::new(root).find_file("Weather")?;
        let folder = WeatherFolder::read(&dir).ok()?;
        (!folder.presets.is_empty()).then(|| Self::from_folder(root.to_path_buf(), folder))
    }

    /// The install root the presets were read from.
    #[must_use]
    pub fn root(&self) -> Option<&std::path::Path> {
        self.root.as_deref()
    }

    /// Whether presets of an install are listed (else the built-in ones).
    #[must_use]
    pub fn is_installed(&self) -> bool {
        !self.installed.is_empty()
    }

    /// The cloud catalogue.
    #[must_use]
    pub fn cloud_types(&self) -> &CloudTypes {
        &self.clouds
    }

    /// The ids offered to the user, in display order.
    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        if self.is_installed() {
            self.installed.iter().map(|p| p.id.clone()).collect()
        } else {
            BUILTIN_IDS.iter().map(|s| (*s).to_owned()).collect()
        }
    }

    pub(super) fn installed(&self, id: &str) -> Option<&PresetFile> {
        self.installed.iter().find(|p| p.id == id).or_else(|| {
            self.installed
                .iter()
                .find(|p| p.id.eq_ignore_ascii_case(id))
        })
    }

    /// The id that stands for `id` in the list: an installed preset's own id, the installed
    /// alias of a built-in id, the built-in id itself (no install, or no close preset) — an
    /// unknown id counts as the built-in id its words suggest ([`legacy_id`]).
    #[must_use]
    pub fn canonical_id(&self, id: &str) -> String {
        if let Some(p) = self.installed(id.trim()) {
            return p.id.clone();
        }
        let legacy = legacy_id(id);
        alias_of(legacy)
            .and_then(|a| self.installed(a))
            .map_or_else(|| legacy.to_owned(), |p| p.id.clone())
    }

    /// The preset file behind an id (installed, alias or built-in).
    #[must_use]
    pub fn file_of(&self, id: &str) -> WeatherFile {
        let canonical = self.canonical_id(id);
        self.installed(&canonical)
            .map_or_else(|| builtin_preset(&canonical), |p| p.weather.clone())
    }
}
