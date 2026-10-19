//! Weather of the session: the preset catalogue, the resolved [`WeatherConditions`] and the pure
//! functions that turn conditions into effect parameters (light scales, fog, sky veil, clouds,
//! precipitation, wet and snowy surfaces).
//!
//! The presets come from the install's `Weather` folder ([`opts_core::formats::weather`]); five
//! built-in presets (the ids of earlier versions) cover a missing install and old saves. The
//! rendering systems live in the game (`opts_game::in_game::weather`); everything here is
//! deterministic and tested without a window. Design: `docs/src/architecture/weather.md`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use opts_core::formats::weather::{
    omsi_language, CloudCover, CloudLayer, CloudTypes, Fog, PrecipKind, PresetFile, WeatherFile,
    WeatherFolder,
};

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

/// What a preset does to the sky, for classification (fallback light, icons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherKind {
    /// Clear sky.
    Clear,
    /// Overcast.
    Overcast,
    /// Rain.
    Rain,
    /// Snow.
    Snow,
    /// Fog.
    Fog,
}

impl WeatherKind {
    /// The kind of a preset id, by the words it contains (`rain`, `snow`, `fog`, `overcast`);
    /// anything else is clear.
    #[must_use]
    pub fn of_preset(id: &str) -> Self {
        let lower = id.to_lowercase();
        if lower.contains("snow") {
            Self::Snow
        } else if lower.contains("rain") {
            Self::Rain
        } else if lower.contains("fog") {
            Self::Fog
        } else if lower.contains("overcast") {
            Self::Overcast
        } else {
            Self::Clear
        }
    }

    /// Whether the sky is covered (the fallback light turns diffuse).
    #[must_use]
    pub fn overcast(self) -> bool {
        self != Self::Clear
    }
}

/// A cloud layer of the resolved conditions.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudSpec {
    /// Type name (`Cumulus 1`).
    pub name: String,
    /// Texture file below `Texture/`; `None` when `clouds.cfg` does not know the type.
    pub texture: Option<String>,
    /// Edge length of one texture repeat in metres.
    pub size_m: f32,
    /// Base height in metres above the camera's ground reference.
    pub base_m: f32,
    /// How the layer is drawn.
    pub cover: CloudCover,
    /// Sky coverage `0..=1` (overcast 1; scattered 0.25 / 0.4 / 0.55 for `Cumulus 1/2/3` ❓).
    pub coverage: f32,
}

/// The values scripts can read, besides the temperature (`Weather_Temperature`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeatherVars {
    /// `Weather_AbsHum` in g/m³.
    pub abs_hum: f32,
    /// `PrecipType`: 0 none, 1 rain, 2 snow.
    pub precip_type: f32,
    /// `PrecipRate`: 0 … 1.
    pub precip_rate: f32,
    /// Wind direction (from), degrees; no script variable is documented for it.
    pub wind_from_deg: f32,
    /// Wind speed in m/s; no script variable is documented for it.
    pub wind_speed: f32,
    /// Pressure in hPa; no script variable is documented for it.
    pub pressure_hpa: f32,
    /// Fog visibility in metres; no script variable is documented for it.
    pub fog_visibility_m: f32,
}

/// The weather of the session, resolved from a preset.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherConditions {
    /// Fog visibility in metres.
    pub fog_visibility_m: f32,
    /// Fog thickness factor `0..=1`.
    pub fog_density: f32,
    /// Direction the wind comes from, degrees clockwise from north.
    pub wind_from_deg: f32,
    /// Wind speed in m/s.
    pub wind_speed: f32,
    /// Air temperature in °C (`Weather_Temperature`).
    pub temperature_c: f32,
    /// Dew point in °C.
    pub dew_point_c: f32,
    /// Pressure in hPa.
    pub pressure_hpa: f32,
    /// The cloud layer, if any.
    pub cloud: Option<CloudSpec>,
    /// What falls.
    pub precip: PrecipKind,
    /// Precipitation rate `0..=1` (the intensity of the file over 255); 0 when dry.
    pub precip_rate: f32,
    /// `[groundwet]` as in the file.
    pub ground_wet: [f32; 3],
    /// Snow lies on the ground.
    pub snow: bool,
    /// Snow lies on the roads.
    pub snow_on_road: bool,
}

impl Default for WeatherConditions {
    fn default() -> Self {
        Self::from_file(&WeatherFile::default(), &CloudTypes::default())
    }
}

impl WeatherConditions {
    /// The conditions of a preset file, with the cloud type looked up in `clouds`.
    #[must_use]
    pub fn from_file(w: &WeatherFile, clouds: &CloudTypes) -> Self {
        let cloud = w.clouds.as_ref().map(|layer| {
            let known = clouds.find(&layer.kind);
            let cover = known.map_or_else(
                || {
                    if layer.kind.to_ascii_lowercase().contains("ovc")
                        || layer.kind.to_ascii_lowercase().contains("overcast")
                    {
                        CloudCover::Overcast
                    } else {
                        CloudCover::Scattered
                    }
                },
                |t| t.cover,
            );
            let digit = layer
                .kind
                .rsplit(' ')
                .next()
                .and_then(|d| d.parse::<f32>().ok())
                .unwrap_or(2.0);
            CloudSpec {
                name: layer.kind.clone(),
                texture: known.map(|t| t.texture.clone()),
                size_m: known.map_or(2000.0, |t| t.size_m),
                base_m: layer.base_m.max(0.0),
                cover,
                coverage: match cover {
                    CloudCover::Overcast => 1.0,
                    CloudCover::Scattered => (0.1 + 0.15 * digit).clamp(0.25, 0.6),
                },
            }
        });
        let precip = w.precip.kind;
        Self {
            fog_visibility_m: w.fog.visibility_m.max(1.0),
            fog_density: w.fog.density.clamp(0.0, 1.0),
            wind_from_deg: w.wind.direction_deg.rem_euclid(360.0),
            wind_speed: w.wind.speed.max(0.0),
            temperature_c: w.temp.celsius,
            dew_point_c: w.temp.dew_point_c.min(w.temp.celsius),
            pressure_hpa: w.pressure_hpa,
            cloud,
            precip,
            precip_rate: if precip == PrecipKind::None {
                0.0
            } else {
                (w.precip.intensity / 255.0).clamp(0.0, 1.0)
            },
            ground_wet: w.ground_wet,
            snow: w.snow,
            snow_on_road: w.snow_on_road,
        }
    }

    /// The classification used by the fallback light and the icons.
    #[must_use]
    pub fn kind(&self) -> WeatherKind {
        if self.precip == PrecipKind::Snow {
            WeatherKind::Snow
        } else if self.precip == PrecipKind::Rain {
            WeatherKind::Rain
        } else if self.fog_visibility_m < 1000.0 {
            WeatherKind::Fog
        } else if self.cloud.as_ref().is_some_and(|c| c.coverage >= 0.5) {
            WeatherKind::Overcast
        } else {
            WeatherKind::Clear
        }
    }

    /// Absolute humidity in g/m³ from the temperature and the dew point (Magnus formula).
    #[must_use]
    pub fn abs_humidity(&self) -> f32 {
        absolute_humidity(self.temperature_c, self.dew_point_c)
    }

    /// The direction the wind blows **towards**, as a unit vector on the ground in Bevy axes
    /// (x east, z south; north is −z).
    #[must_use]
    pub fn wind_to(&self) -> Vec2 {
        let b = self.wind_from_deg.to_radians();
        Vec2::new(-b.sin(), b.cos())
    }

    /// The values scripts and the host API read.
    #[must_use]
    pub fn vars(&self) -> WeatherVars {
        WeatherVars {
            abs_hum: self.abs_humidity(),
            precip_type: f32::from(self.precip.code()),
            precip_rate: self.precip_rate,
            wind_from_deg: self.wind_from_deg,
            wind_speed: self.wind_speed,
            pressure_hpa: self.pressure_hpa,
            fog_visibility_m: self.fog_visibility_m,
        }
    }
}

/// Absolute humidity (g/m³) of air at `temp_c` whose dew point is `dew_c`: the vapour pressure at
/// the dew point (Magnus) over the gas law at the air temperature.
#[must_use]
pub fn absolute_humidity(temp_c: f32, dew_c: f32) -> f32 {
    let dew = dew_c.min(temp_c);
    let e_hpa = 6.112 * (17.62 * dew / (243.12 + dew)).exp();
    216.7 * e_hpa / (273.15 + temp_c)
}

// ---------------------------------------------------------------------------------------------
// Catalogue
// ---------------------------------------------------------------------------------------------

/// The presets the game can offer: the install's, or the five built-in ones.
///
/// A resource; the game fills it from the install's `Weather` folder when the root is known.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct WeatherCatalogue {
    root: Option<PathBuf>,
    installed: Vec<PresetFile>,
    clouds: CloudTypes,
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

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

/// How many rain/snow quads the precipitation may use at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrecipitationQuality {
    /// No precipitation is drawn.
    Off,
    /// 1500 quads.
    Low,
    /// 3000 quads.
    #[default]
    Medium,
    /// 6000 quads.
    High,
}

impl PrecipitationQuality {
    /// The stored value.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The quality of a stored value.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "off" => Some(Self::Off),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// The quad budget.
    #[must_use]
    pub fn budget(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 1500,
            Self::Medium => 3000,
            Self::High => 6000,
        }
    }
}

/// Which weather effects are drawn (the settings `graphics.weather_*`; the game writes it).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherEffects {
    /// Distance fog.
    pub fog: bool,
    /// Cloud layer.
    pub clouds: bool,
    /// Rain and snow.
    pub precipitation: PrecipitationQuality,
    /// Wet and snowy ground and roads.
    pub wet_ground: bool,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            fog: true,
            clouds: true,
            precipitation: PrecipitationQuality::default(),
            wet_ground: true,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pure derivations
// ---------------------------------------------------------------------------------------------

/// Visibility above which no distance fog is drawn (metres).
pub const FOG_MAX_VISIBILITY_M: f32 = 15_000.0;

/// How the weather scales the sun's illuminance and the ambient brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightScales {
    /// Multiplier of the direct sunlight.
    pub sun: f32,
    /// Multiplier of the ambient light.
    pub ambient: f32,
}

/// How foggy the air is, `0` (10 km visibility or more) to `1` (a few metres).
#[must_use]
pub fn fogginess(c: &WeatherConditions) -> f32 {
    (1.0 - c.fog_visibility_m / 10_000.0).clamp(0.0, 1.0)
}

/// The share of the sky that is covered for lighting: the larger of the clouds, the
/// precipitation (rain and snow come out of a covered sky) and the thick fog.
#[must_use]
pub fn effective_cover(c: &WeatherConditions) -> f32 {
    let clouds = c.cloud.as_ref().map_or(0.0, |cl| cl.coverage);
    let precip = if c.precip == PrecipKind::None || c.precip_rate <= 0.0 {
        0.0
    } else {
        0.5 + 0.5 * c.precip_rate
    };
    let fog = fogginess(c) * (0.5 + 0.5 * c.fog_density);
    clouds.max(precip).max(fog).clamp(0.0, 1.0)
}

/// **The one function through which weather scales the sun and the ambient light.** A covered sky
/// (cover 1) gives the sun 0.55 and the ambient light 0.8, a clear one 1 and 1.
#[must_use]
pub fn light_scales(c: &WeatherConditions) -> LightScales {
    light_scales_for_cover(effective_cover(c))
}

/// [`light_scales`] for a given share of covered sky `0..=1` (the rule itself, so the sky
/// description [`crate::environment::WeatherSky`] shares it).
#[must_use]
pub fn light_scales_for_cover(cover: f32) -> LightScales {
    let cover = cover.clamp(0.0, 1.0);
    LightScales {
        sun: 1.0 - 0.45 * cover,
        ambient: 1.0 - 0.2 * cover,
    }
}

/// Distance fog: exponential-squared density per metre, `None` when the air is clear.
///
/// The density makes an object vanish at the visibility: the fog's contrast falls to a threshold
/// at that distance (2 % for a full-thickness layer, up to 10 % for a thin one, so thin fog is
/// lighter at the same visibility).
#[must_use]
pub fn fog_density(c: &WeatherConditions) -> Option<f32> {
    fog_density_for(c, c.fog_visibility_m)
}

/// [`fog_density`] for another visibility than the preset's (the ground-fog layer raises it with
/// the camera's height, see [`ground_fog`]).
#[must_use]
pub fn fog_density_for(c: &WeatherConditions, visibility_m: f32) -> Option<f32> {
    if visibility_m >= FOG_MAX_VISIBILITY_M {
        return None;
    }
    let contrast = 0.02 + 0.08 * (1.0 - c.fog_density);
    Some((-contrast.ln()).sqrt() / visibility_m.max(1.0))
}

// ---------------------------------------------------------------------------------------------
// Ground-fog layer
// ---------------------------------------------------------------------------------------------

/// Presets with a visibility **at or below** this many metres are ground fog (a low layer);
/// anything thinner (dusk fog 500 m, storm 700 m, haze) is fog everywhere, as before. ❓ The
/// corpus has one ground-fog preset (`Bodennebel`, 75 m); `.owt` has no "layer" flag.
pub const FOG_LAYER_MAX_VISIBILITY_M: f32 = 150.0;
/// Layer thickness as a share of the visibility ❓ (`Bodennebel`: 0.4 × 75 m = 30 m).
pub const FOG_LAYER_THICKNESS_PER_VISIBILITY: f32 = 0.4;
/// Thinnest and thickest layer in metres.
pub const FOG_LAYER_THICKNESS_RANGE_M: (f32, f32) = (25.0, 200.0);
/// Below this share of the layer thickness the camera is inside the layer (full fog).
pub const FOG_LAYER_INSIDE: f32 = 0.5;
/// Above this multiple of the thickness the camera is clear of the layer.
pub const FOG_LAYER_ABOVE: f32 = 2.0;
/// Looking up by this many degrees (from above the layer) takes the fog off what is seen.
pub const FOG_LAYER_LOOK_UP_DEG: f32 = 15.0;
/// How much farther the visibility reaches when looking fully up from above the layer.
pub const FOG_LAYER_LOOK_UP_GAIN: f32 = 3.0;

/// The thickness in metres of the ground-fog layer of the conditions, `None` when the preset is
/// not ground fog (visibility above [`FOG_LAYER_MAX_VISIBILITY_M`]).
///
/// The `.owt` second value is a unitless factor `0.5 … 1`, not metres (📖 environment.md), so the
/// thickness is a documented estimate: the visibility times
/// [`FOG_LAYER_THICKNESS_PER_VISIBILITY`], scaled by `0.5 + 0.5 ·` that factor, clamped to
/// [`FOG_LAYER_THICKNESS_RANGE_M`].
#[must_use]
pub fn fog_layer_thickness(c: &WeatherConditions) -> Option<f32> {
    if c.fog_visibility_m > FOG_LAYER_MAX_VISIBILITY_M {
        return None;
    }
    let (lo, hi) = FOG_LAYER_THICKNESS_RANGE_M;
    Some(
        (c.fog_visibility_m * FOG_LAYER_THICKNESS_PER_VISIBILITY * (0.5 + 0.5 * c.fog_density))
            .clamp(lo, hi),
    )
}

/// What the ground-fog layer does to the fog and the sky for the camera's height and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogParams {
    /// Visibility in metres to feed the distance fog (the preset's, raised above the layer).
    pub visibility_m: f32,
    /// How far the camera is clear of the layer, `0` (inside) … `1` (above): the share by which
    /// the sky's fog veil is taken away.
    pub sky_clear: f32,
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The fog for a camera `height_m` above the ground, looking `pitch_deg` above the horizon
/// (negative: down), in a fog layer of `thickness_m`.
///
/// Bevy's `DistanceFog` knows only the distance to a fragment. For a ground fragment at distance
/// `d` seen from height `h > T` the ray runs `T·d/h` through the layer, so the layer equals
/// distance fog with the visibility multiplied by `h/T`, whatever the pitch. This is exact for
/// the ground (the exponential-squared shape keeps the near ground lighter than the exact
/// exponential would) and approximate for objects above it. The pitch term: looking up from above
/// the layer the ray never enters it, so the visibility grows by up to
/// `FOG_LAYER_LOOK_UP_GAIN + 1` times. The sky is not distance fog at all; `sky_clear` tells
/// `sky.rs` how much of its veil to take away.
///
/// Inside the layer (`h ≤ 0.5·T`) the result is the preset's own fog; between `0.5·T` and `2·T`
/// the factor `1 … 2` follows a smoothstep, and from `2·T` up it is `h/T`; all continuous and
/// monotonic in `h`.
#[must_use]
pub fn ground_fog(visibility_m: f32, thickness_m: f32, height_m: f32, pitch_deg: f32) -> FogParams {
    let t = thickness_m.max(1.0);
    let h = height_m.max(0.0);
    let above = smoothstep(FOG_LAYER_INSIDE * t, FOG_LAYER_ABOVE * t, h);
    let ratio = if h >= FOG_LAYER_ABOVE * t {
        h / t
    } else {
        1.0 + above
    };
    let up = smoothstep(0.0, FOG_LAYER_LOOK_UP_DEG, pitch_deg);
    let scale = ratio * (1.0 + FOG_LAYER_LOOK_UP_GAIN * above * up);
    FogParams {
        visibility_m: (visibility_m * scale).min(FOG_MAX_VISIBILITY_M - 1.0),
        sky_clear: above,
    }
}

/// Where the camera is relative to the ground-fog layer; written by the game each frame, read by
/// the fog, the sky and `sim.get`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct GroundFogView {
    /// Camera height above the terrain under it, metres.
    pub height_m: f32,
    /// Camera pitch above the horizon, degrees.
    pub pitch_deg: f32,
    /// Layer thickness in metres (`None`: this weather is not ground fog or the effect is off).
    pub layer_m: Option<f32>,
    /// The visibility the distance fog gets now.
    pub effective_visibility_m: f32,
    /// How much of the sky's veil is taken away, `0..=1`.
    pub sky_clear: f32,
}

/// The [`GroundFogView`] for the conditions and a camera pose (`fog_on`: the fog effect is on).
#[must_use]
pub fn ground_fog_view(
    c: &WeatherConditions,
    fog_on: bool,
    height_m: f32,
    pitch_deg: f32,
) -> GroundFogView {
    let layer = fog_on.then(|| fog_layer_thickness(c)).flatten();
    match layer {
        Some(t) => {
            let p = ground_fog(c.fog_visibility_m, t, height_m, pitch_deg);
            GroundFogView {
                height_m,
                pitch_deg,
                layer_m: Some(t),
                effective_visibility_m: p.visibility_m,
                sky_clear: p.sky_clear,
            }
        }
        None => GroundFogView {
            height_m,
            pitch_deg,
            layer_m: None,
            effective_visibility_m: c.fog_visibility_m,
            sky_clear: 0.0,
        },
    }
}

/// Largest saturation `(max - min) / max` of the colour of a full-thickness fog; thinner fog
/// allows more up to the horizon's own colour in clear air.
pub const FOG_MAX_SATURATION: f32 = 0.2;

/// Fog colour: the sky's horizon colour with most of its chroma taken out, in sRGB components
/// `0..=1`, as the horizon texture holds them.
///
/// Real fog is grey-white: droplets scatter all wavelengths alike, so the hue of a sunset horizon
/// (orange, olive) must not tint it. The share of the chroma that goes is
/// `thick * (0.6 + 0.35 * thick)` with `thick` the fogginess (or half the cover), about 94 % in
/// fog of 100 m, and the saturation is capped at `1 - 0.8 * thick` (0.2 in thick fog). The
/// luminance follows the horizon (so the fog is dark at night and as dark under cloud as the
/// horizon texture is), scaled by 0.95. In clear air the colour is the horizon's.
#[must_use]
pub fn fog_color(horizon: [f32; 3], c: &WeatherConditions) -> [f32; 3] {
    let luma = |v: [f32; 3]| (v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11).max(0.0);
    let grey = luma(horizon) * 0.95;
    let thick = fogginess(c).max(effective_cover(c) * 0.5).clamp(0.0, 1.0);
    let t = thick * (0.6 + 0.35 * thick);
    let mixed = horizon.map(|v| v + (grey - v) * t);
    let hi = mixed.iter().copied().fold(0.0_f32, f32::max);
    let lo = mixed.iter().copied().fold(f32::MAX, f32::min);
    let cap = 1.0 - (1.0 - FOG_MAX_SATURATION) * thick;
    if hi > 1e-6 && (hi - lo) / hi > cap {
        // Pull towards the colour's own luminance until the saturation is at the cap.
        let mean = luma(mixed);
        let k = (cap * mean / ((hi - lo) - cap * (hi - mean))).clamp(0.0, 1.0);
        return mixed.map(|v| mean + (v - mean) * k);
    }
    mixed
}

/// How far the sky texture is veiled towards the fog colour at height `v` (`0` zenith … `1`
/// horizon), `0..=1`: fog covers all of it, an overcast sky greys it mostly towards the horizon
/// (the cloud layer reaches only so far).
#[must_use]
pub fn sky_veil(c: &WeatherConditions, v: f32) -> f32 {
    sky_veil_clear(c, v, 0.0)
}

/// [`sky_veil`] with the fog lifted for a camera above the ground-fog layer
/// ([`GroundFogView::sky_clear`]): the fog part of the veil falls to nothing at the zenith and to
/// half at the horizon (a hazy rim: `0.5 · (1 − elevation/8°)²` below 8° of elevation, the
/// texture row `v` being `1 − elevation/90°`); the overcast part is unchanged.
#[must_use]
pub fn sky_veil_clear(c: &WeatherConditions, v: f32, sky_clear: f32) -> f32 {
    let overcast = c
        .cloud
        .as_ref()
        .filter(|cl| cl.cover == CloudCover::Overcast)
        .map_or(0.0, |_| 0.45 * (0.3 + 0.7 * v.clamp(0.0, 1.0)));
    let elevation_deg = (1.0 - v.clamp(0.0, 1.0)) * 90.0;
    let rim = 0.5 * (1.0 - elevation_deg / 8.0).clamp(0.0, 1.0).powi(2);
    let lift = 1.0 - sky_clear.clamp(0.0, 1.0) * (1.0 - rim);
    (fogginess(c) * 0.9 * lift).max(overcast)
}

/// Brightness of the unlit cloud layer: dark at night, full at day, a little darker under rain.
#[must_use]
pub fn cloud_tint(c: &WeatherConditions, sun_alt_deg: f32) -> f32 {
    let t = ((sun_alt_deg + 8.0) / 18.0).clamp(0.0, 1.0);
    let day = t * t * (3.0 - 2.0 * t);
    let dim = 1.0 - 0.3 * c.precip_rate - 0.15 * fogginess(c);
    (0.04 + 0.96 * day) * dim
}

/// The colours and brightnesses of the weather effects, **scene-linear** (what the camera
/// renders before tonemapping). The one place that turns the sky's sRGB horizon colour and the
/// time of day into them, so a switch of the camera to HDR (an exposure factor) changes this
/// function only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherLook {
    /// Fog colour, linear RGB.
    pub fog: [f32; 3],
    /// Brightness of the cloud layer, linear `0..=1`.
    pub cloud: f32,
    /// Brightness of rain and snow, linear `0.25..=1`.
    pub precip: f32,
}

/// The [`WeatherLook`] for the conditions, the sky's horizon colour (sRGB `0..=1`, as the sky
/// texture holds it) and the sun's elevation.
#[must_use]
pub fn weather_look(
    c: &WeatherConditions,
    horizon_srgb: [f32; 3],
    sun_alt_deg: f32,
) -> WeatherLook {
    let fog_srgb = fog_color(horizon_srgb, c);
    let lin = Color::srgb(fog_srgb[0], fog_srgb[1], fog_srgb[2]).to_linear();
    let cloud = cloud_tint(c, sun_alt_deg);
    WeatherLook {
        fog: [lin.red, lin.green, lin.blue],
        cloud,
        precip: 0.25 + 0.75 * cloud,
    }
}

/// Cloud drift in metres per second on the ground plane (Bevy axes): the wind carries the clouds
/// three times faster than it blows at the ground.
#[must_use]
pub fn cloud_drift(c: &WeatherConditions) -> Vec2 {
    c.wind_to() * c.wind_speed * 3.0
}

/// Rain or snow around the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecipPlan {
    /// Rain or snow.
    pub kind: PrecipKind,
    /// Quads in use, at most the quality's budget.
    pub quads: u32,
    /// Fall speed in m/s.
    pub fall_speed: f32,
    /// Horizontal velocity in m/s (the wind, damped).
    pub drift: Vec2,
    /// Width and length of a quad in metres.
    pub size: Vec2,
}

/// The precipitation to draw: none for dry weather, a budget of `0` or a tiny rate.
#[must_use]
pub fn precip_plan(c: &WeatherConditions, budget: u32) -> Option<PrecipPlan> {
    if c.precip == PrecipKind::None || c.precip_rate <= 0.0 || budget == 0 {
        return None;
    }
    let share = 0.15 + 0.85 * c.precip_rate;
    let quads = ((budget as f32 * share).round() as u32).clamp(1, budget);
    let (fall, damp, size) = match c.precip {
        PrecipKind::Rain => (9.0, 0.6, Vec2::new(0.025, 0.8)),
        _ => (1.4, 0.9, Vec2::new(0.08, 0.08)),
    };
    let mut drift = c.wind_to() * c.wind_speed * damp;
    let cap = fall * 1.5;
    if drift.length() > cap {
        drift = drift.normalize() * cap;
    }
    Some(PrecipPlan {
        kind: c.precip,
        quads,
        fall_speed: fall,
        drift,
        size,
    })
}

/// Wet and snowy surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SurfaceState {
    /// Wetness `0..=1` (darker, smoother).
    pub wet: f32,
    /// Puddle coverage `0..=1` on the ground.
    pub puddles: f32,
    /// Snow cover of the ground `0..=1`.
    pub snow_ground: f32,
    /// Snow cover of the roads `0..=1`.
    pub snow_road: f32,
}

/// Ground wetness and snow from `[groundwet]` and the snow flags. The meaning of the three values
/// is a guess (second / 255 is the wetness, third / 120 the puddles ❓); snow covers part of the
/// wetness.
#[must_use]
pub fn surface_state(c: &WeatherConditions) -> SurfaceState {
    let snow_ground = if c.snow { 0.85 } else { 0.0 };
    let snow_road = if c.snow_on_road { 0.7 } else { 0.0 };
    SurfaceState {
        wet: (c.ground_wet[1] / 255.0).clamp(0.0, 1.0) * (1.0 - 0.6 * snow_ground),
        puddles: 0.45 * (c.ground_wet[2] / 120.0).clamp(0.0, 1.0) * (1.0 - snow_ground),
        snow_ground,
        snow_road,
    }
}

/// Colour and roughness of a road material under the surface state: darker and smoother when
/// wet, tinted towards snow with snow on the road. `base` is linear RGB.
#[must_use]
pub fn road_surface(base: [f32; 3], roughness: f32, s: &SurfaceState) -> ([f32; 3], f32) {
    let dark = 1.0 - 0.3 * s.wet;
    let snow = [0.92, 0.94, 0.97];
    let c = std::array::from_fn(|i| {
        let wet = base[i] * dark;
        wet + (snow[i] - wet) * s.snow_road
    });
    (c, (roughness * (1.0 - 0.55 * s.wet)).clamp(0.04, 1.0))
}

#[cfg(test)]
mod tests;
