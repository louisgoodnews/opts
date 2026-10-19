//! The weather catalogue: built-in presets, aliases and lookup.

use super::*;

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

/// The built-in preset value of a built-in id (behaves as the five presets always did: clear,
/// overcast and wet/snowy/foggy skies at 15 °C, −3 °C with snow).
#[must_use]
pub fn builtin_preset(id: &str) -> WeatherFile {
    let mut w = WeatherFile::default();
    let overcast = |base_m: f32| {
        Some(CloudLayer {
            kind: "Overcast 1".to_owned(),
            base_m,
        })
    };
    match legacy_id(id) {
        "Overcast Autumn" => {
            w.clouds = overcast(300.0);
            w.temp.dew_point_c = 11.0;
            w.fog.visibility_m = 12_000.0;
        }
        "Rainy Afternoon" => {
            w.clouds = overcast(200.0);
            w.precip.kind = PrecipKind::Rain;
            w.precip.intensity = 130.0;
            w.ground_wet = [78.0, 255.0, 60.0];
            w.temp.dew_point_c = 13.0;
            w.fog.visibility_m = 6000.0;
            w.wind.direction_deg = 230.0;
            w.wind.speed = 4.0;
        }
        "Winter Snowfall" => {
            w.clouds = overcast(250.0);
            w.precip.kind = PrecipKind::Snow;
            w.precip.intensity = 150.0;
            w.snow = true;
            w.snow_on_road = true;
            w.ground_wet = [1.7, 255.0, 0.0];
            w.temp.celsius = -3.0;
            w.temp.dew_point_c = -4.0;
            w.fog.visibility_m = 3000.0;
            w.wind.direction_deg = 300.0;
            w.wind.speed = 1.5;
        }
        "Autumn Morning Fog" => {
            w.clouds = overcast(100.0);
            w.fog = Fog {
                visibility_m: 300.0,
                density: 1.0,
            };
            w.temp.dew_point_c = 15.0;
            w.ground_wet = [30.0, 120.0, 0.0];
        }
        _ => {}
    }
    w
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

    fn installed(&self, id: &str) -> Option<&PresetFile> {
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

    /// The conditions of the preset behind `id`.
    #[must_use]
    pub fn resolve(&self, id: &str) -> WeatherConditions {
        WeatherConditions::from_file(&self.file_of(id), &self.clouds)
    }

    /// The localised name of an installed preset for a UI locale (`de`, `en`); `None` for the
    /// built-in presets (the UI has message keys for them).
    #[must_use]
    pub fn display_name(&self, id: &str, locale: &str) -> Option<String> {
        let canonical = self.canonical_id(id);
        self.installed(&canonical)
            .map(|p| p.name(omsi_language(locale)))
    }

    /// The localised description of an installed preset.
    #[must_use]
    pub fn description(&self, id: &str, locale: &str) -> Option<String> {
        let canonical = self.canonical_id(id);
        self.installed(&canonical)
            .map(|p| p.description(omsi_language(locale)))
            .filter(|d| !d.is_empty())
    }

    /// The listed id a user-typed name stands for: an id, its slug, a localised name or a word of
    /// one of them (`rain`, `snow`), case-insensitively. Old built-in ids work as well.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<String> {
        let n = name.trim().to_lowercase();
        if n.is_empty() {
            return None;
        }
        let ids = self.ids();
        let slug = |s: &str| {
            s.to_lowercase()
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '#' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_owned()
        };
        let texts = |id: &str| -> BTreeSet<String> {
            let mut set = BTreeSet::from([id.to_lowercase()]);
            if let Some(p) = self.installed(id) {
                set.extend(p.texts.values().map(|t| t.name.to_lowercase()));
                set.insert(p.weather.clean_name().to_lowercase());
            }
            set
        };
        let exact = ids.iter().find(|id| {
            texts(id).contains(&n) || slug(id) == slug(&n) || {
                let names = texts(id);
                names.iter().any(|t| slug(t) == slug(&n))
            }
        });
        if let Some(id) = exact {
            return Some(id.clone());
        }
        // An old built-in id, spelled out.
        if let Some(b) = BUILTIN_IDS
            .iter()
            .find(|b| b.to_lowercase() == n || slug(b) == slug(&n))
        {
            return Some(self.canonical_id(b));
        }
        ids.iter()
            .find(|id| {
                texts(id).iter().any(|t| {
                    t.split(|c: char| !c.is_alphanumeric())
                        .any(|w| w.starts_with(&n))
                })
            })
            .cloned()
            .or_else(|| {
                // A word of a built-in id (`rain`, `snow`, `fog`, `clear`).
                BUILTIN_IDS
                    .iter()
                    .find(|b| b.to_lowercase().split(' ').any(|w| w.starts_with(&n)))
                    .map(|b| self.canonical_id(b))
            })
    }
}
