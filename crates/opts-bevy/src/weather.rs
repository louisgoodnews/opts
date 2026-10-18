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

/// Fog colour: the sky's horizon colour pulled towards grey in thick fog, in linear sRGB
/// components `0..=1`. The horizon colour already follows the time of day, so the fog is dark at
/// night.
#[must_use]
pub fn fog_color(horizon: [f32; 3], c: &WeatherConditions) -> [f32; 3] {
    let grey = (horizon[0] * 0.3 + horizon[1] * 0.59 + horizon[2] * 0.11) * 0.95;
    let t = 0.6 * fogginess(c).max(effective_cover(c) * 0.5);
    horizon.map(|v| v + (grey - v) * t)
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
mod tests {
    use super::*;
    use opts_core::formats::weather::{CloudType, WeatherText};

    fn clouds() -> CloudTypes {
        CloudTypes(vec![
            CloudType {
                name: "Cumulus 1".into(),
                texture: "Cumulus_1.tga".into(),
                size_m: 2000.0,
                cover: CloudCover::Scattered,
            },
            CloudType {
                name: "Overcast 1".into(),
                texture: "OVC_1.bmp".into(),
                size_m: 1000.0,
                cover: CloudCover::Overcast,
            },
        ])
    }

    fn preset(id: &str, w: WeatherFile, en: &str, de: &str) -> PresetFile {
        let text = |n: &str| WeatherText {
            name: n.into(),
            description: format!("{n} text"),
        };
        PresetFile {
            id: id.into(),
            weather: w,
            texts: [("ENG".to_owned(), text(en)), ("DEU".to_owned(), text(de))].into(),
        }
    }

    fn installed() -> WeatherCatalogue {
        let rain = WeatherFile {
            precip: opts_core::formats::weather::Precip {
                kind: PrecipKind::Rain,
                intensity: 173.0,
                extra: [0.0; 3],
            },
            clouds: Some(CloudLayer {
                kind: "Overcast 1".into(),
                base_m: 200.0,
            }),
            fog: Fog {
                visibility_m: 200.0,
                density: 1.0,
            },
            ..WeatherFile::default()
        };
        let mut presets = vec![
            preset(
                "#CAVOK",
                WeatherFile::default(),
                "Best Weather",
                "Bestes Wetter",
            ),
            preset("Schmuddelwetter", rain, "Grey and wet", "Schmuddelwetter"),
            preset(
                "Sommerlich",
                WeatherFile::default(),
                "Summerlike",
                "Sommerlich",
            ),
        ];
        presets.sort_by(|a, b| a.id.cmp(&b.id));
        WeatherCatalogue::from_folder(
            PathBuf::from("/omsi"),
            WeatherFolder {
                presets,
                clouds: clouds(),
            },
        )
    }

    #[test]
    fn builtin_presets_keep_the_light_and_temperature_of_the_old_kinds() {
        let cat = WeatherCatalogue::builtin();
        for (id, sun, ambient, temp) in [
            ("Clear Summer Day", 1.0, 1.0, 15.0),
            ("Overcast Autumn", 0.55, 0.8, 15.0),
            ("Rainy Afternoon", 0.55, 0.8, 15.0),
            ("Winter Snowfall", 0.55, 0.8, -3.0),
            ("Autumn Morning Fog", 0.55, 0.8, 15.0),
        ] {
            let c = cat.resolve(id);
            let l = light_scales(&c);
            assert!((l.sun - sun).abs() < 1e-3, "{id} sun {}", l.sun);
            assert!(
                (l.ambient - ambient).abs() < 1e-3,
                "{id} ambient {}",
                l.ambient
            );
            assert_eq!(c.temperature_c, temp, "{id}");
            assert_eq!(c.kind(), WeatherKind::of_preset(id), "{id}");
        }
    }

    #[test]
    fn without_an_install_the_five_ids_are_listed() {
        let cat = WeatherCatalogue::builtin();
        assert_eq!(cat.ids(), BUILTIN_IDS.map(String::from));
        assert!(!cat.is_installed());
        assert_eq!(cat.canonical_id("Rainy Afternoon"), "Rainy Afternoon");
        // An unknown id behaves like the words it contains, else like clear weather.
        assert_eq!(cat.canonical_id("Heavy snow day"), "Winter Snowfall");
        assert_eq!(cat.canonical_id("Thunderstorm & Gale"), "Clear Summer Day");
        assert_eq!(cat.canonical_id(""), "Clear Summer Day");
    }

    #[test]
    fn old_ids_alias_to_the_closest_installed_preset() {
        let cat = installed();
        assert_eq!(cat.ids(), ["#CAVOK", "Schmuddelwetter", "Sommerlich"]);
        assert_eq!(cat.canonical_id("Clear Summer Day"), "#CAVOK");
        assert_eq!(cat.canonical_id("Rainy Afternoon"), "Schmuddelwetter");
        // No installed preset in this catalogue is close to snow or fog: the built-in one stays.
        assert_eq!(cat.canonical_id("Winter Snowfall"), "Winter Snowfall");
        // The overcast id has no alias at all.
        assert_eq!(cat.canonical_id("Overcast Autumn"), "Overcast Autumn");
        assert_eq!(cat.canonical_id("sommerlich"), "Sommerlich");
        // The resolved conditions are the installed ones.
        let c = cat.resolve("Rainy Afternoon");
        assert_eq!(c.fog_visibility_m, 200.0);
        assert!((c.precip_rate - 173.0 / 255.0).abs() < 1e-6);
        // And the built-in ones when no alias exists.
        assert_eq!(cat.resolve("Winter Snowfall").temperature_c, -3.0);
    }

    #[test]
    fn names_follow_the_ui_language() {
        let cat = installed();
        assert_eq!(
            cat.display_name("Sommerlich", "de").as_deref(),
            Some("Sommerlich")
        );
        assert_eq!(
            cat.display_name("#CAVOK", "de").as_deref(),
            Some("Bestes Wetter")
        );
        assert_eq!(
            cat.display_name("#CAVOK", "en").as_deref(),
            Some("Best Weather")
        );
        // A language OMSI has no text for falls back to English.
        assert_eq!(
            cat.display_name("#CAVOK", "ja").as_deref(),
            Some("Best Weather")
        );
        assert!(WeatherCatalogue::builtin()
            .display_name("Clear Summer Day", "en")
            .is_none());
    }

    #[test]
    fn find_accepts_ids_slugs_names_words_and_old_ids() {
        let cat = installed();
        assert_eq!(
            cat.find("schmuddelwetter").as_deref(),
            Some("Schmuddelwetter")
        );
        assert_eq!(cat.find("Best Weather").as_deref(), Some("#CAVOK"));
        assert_eq!(cat.find("bestes-wetter").as_deref(), Some("#CAVOK"));
        assert_eq!(cat.find("grey").as_deref(), Some("Schmuddelwetter"));
        assert_eq!(
            cat.find("rainy-afternoon").as_deref(),
            Some("Schmuddelwetter")
        );
        assert_eq!(cat.find("rain").as_deref(), Some("Schmuddelwetter"));
        assert_eq!(cat.find("clear").as_deref(), Some("#CAVOK"));
        assert_eq!(cat.find("hail"), None);
        let b = WeatherCatalogue::builtin();
        assert_eq!(b.find("rain").as_deref(), Some("Rainy Afternoon"));
        assert_eq!(
            b.find("winter-snowfall").as_deref(),
            Some("Winter Snowfall")
        );
        assert_eq!(
            b.find("autumn-morning-fog").as_deref(),
            Some("Autumn Morning Fog")
        );
    }

    #[test]
    fn conditions_from_a_file() {
        let w = WeatherFile {
            clouds: Some(CloudLayer {
                kind: "Cumulus 3".into(),
                base_m: 300.0,
            }),
            ..WeatherFile::default()
        };
        let c = WeatherConditions::from_file(&w, &clouds());
        let cl = c.cloud.unwrap();
        // The type is not in this catalogue: no texture, scattered, coverage from the digit.
        assert_eq!(cl.texture, None);
        assert_eq!(cl.cover, CloudCover::Scattered);
        assert!((cl.coverage - 0.55).abs() < 1e-6);
        let c1 = WeatherConditions::from_file(
            &WeatherFile {
                clouds: Some(CloudLayer {
                    kind: "Cumulus 1".into(),
                    base_m: 100.0,
                }),
                ..WeatherFile::default()
            },
            &clouds(),
        );
        assert_eq!(
            c1.cloud.as_ref().unwrap().texture.as_deref(),
            Some("Cumulus_1.tga")
        );
        assert!((c1.cloud.unwrap().coverage - 0.25).abs() < 1e-6);
    }

    #[test]
    fn humidity_follows_the_dew_point() {
        // 15 °C with a 5 °C dew point: about 6.6 g/m³.
        assert!((absolute_humidity(15.0, 5.0) - 6.56).abs() < 0.1);
        // Saturated air at 20 °C holds about 17.3 g/m³.
        assert!((absolute_humidity(20.0, 20.0) - 17.3).abs() < 0.3);
        // A dew point above the temperature is capped.
        assert_eq!(absolute_humidity(10.0, 30.0), absolute_humidity(10.0, 10.0));
        let c = WeatherConditions::default();
        assert!((c.abs_humidity() - 6.56).abs() < 0.1);
    }

    #[test]
    fn script_variables_of_rain_and_dry_weather() {
        let dry = WeatherConditions::default().vars();
        assert_eq!((dry.precip_type, dry.precip_rate), (0.0, 0.0));
        let rain = installed().resolve("Schmuddelwetter").vars();
        assert_eq!(rain.precip_type, 1.0);
        assert!((rain.precip_rate - 173.0 / 255.0).abs() < 1e-6);
        let snow = WeatherCatalogue::builtin()
            .resolve("Winter Snowfall")
            .vars();
        assert_eq!(snow.precip_type, 2.0);
    }

    #[test]
    fn fog_is_off_when_the_air_is_clear_and_denser_for_shorter_visibility() {
        let mut c = WeatherConditions::default();
        assert_eq!(fog_density(&c), None);
        c.fog_visibility_m = 14_000.0;
        let thin = fog_density(&c).unwrap();
        c.fog_visibility_m = 75.0;
        let thick = fog_density(&c).unwrap();
        assert!(thick > thin * 100.0, "{thin} {thick}");
        // At the visibility the fog's transmittance is the contrast threshold (2 %).
        let transmittance = (-(thick * 75.0).powi(2)).exp();
        assert!((transmittance - 0.02).abs() < 1e-4, "{transmittance}");
        // A thin layer (0.5) at the same visibility is lighter.
        c.fog_density = 0.5;
        assert!(fog_density(&c).unwrap() < thick);
    }

    #[test]
    fn fog_colour_greys_in_thick_fog_and_keeps_the_horizon_in_clear_air() {
        let horizon = [0.5, 0.6, 0.9];
        let clear = WeatherConditions::default();
        assert_eq!(fog_color(horizon, &clear), horizon);
        let mut fog = clear;
        fog.fog_visibility_m = 80.0;
        let f = fog_color(horizon, &fog);
        let spread = |c: [f32; 3]| c[2] - c[0];
        assert!(spread(f) < spread(horizon) * 0.6, "{f:?}");
        // The colour darkens with the horizon at night.
        let night = fog_color([0.02, 0.02, 0.05], &fog);
        assert!(night.iter().all(|v| *v < 0.06));
    }

    #[test]
    fn the_sky_is_veiled_by_fog_everywhere_and_by_overcast_towards_the_horizon() {
        let clear = WeatherConditions::default();
        assert_eq!(sky_veil(&clear, 1.0), 0.0);
        let mut fog = clear;
        fog.fog_visibility_m = 100.0;
        assert!(sky_veil(&fog, 0.0) > 0.85 && sky_veil(&fog, 1.0) > 0.85);
        let over = WeatherCatalogue::builtin().resolve("Overcast Autumn");
        assert!(sky_veil(&over, 0.0) < sky_veil(&over, 1.0));
        assert!(sky_veil(&over, 1.0) <= 0.45 + 1e-6);
    }

    #[test]
    fn light_scales_cover_the_documented_range() {
        let clear = light_scales(&WeatherConditions::default());
        assert_eq!((clear.sun, clear.ambient), (1.0, 1.0));
        let cat = installed();
        let storm = light_scales(&cat.resolve("Schmuddelwetter"));
        assert!(storm.sun < 0.6 && storm.sun >= 0.55 - 1e-3, "{}", storm.sun);
        // Cover never darkens below the overcast values.
        let mut c = cat.resolve("Schmuddelwetter");
        c.precip_rate = 1.0;
        c.fog_visibility_m = 5.0;
        assert!(light_scales(&c).sun >= 0.55 - 1e-3);
    }

    #[test]
    fn precipitation_scales_with_the_rate_and_the_budget() {
        let mut c = WeatherConditions::default();
        assert_eq!(precip_plan(&c, 3000), None);
        c.precip = PrecipKind::Rain;
        c.precip_rate = 0.1;
        let light = precip_plan(&c, 3000).unwrap();
        c.precip_rate = 1.0;
        let heavy = precip_plan(&c, 3000).unwrap();
        assert!(light.quads < heavy.quads);
        assert_eq!(heavy.quads, 3000);
        assert_eq!(precip_plan(&c, 0), None);
        assert!(precip_plan(&c, 1500).unwrap().quads <= 1500);
        c.precip_rate = 0.0;
        assert_eq!(precip_plan(&c, 3000), None, "no rate, no drops");
        // Snow falls slowly.
        c.precip = PrecipKind::Snow;
        c.precip_rate = 0.5;
        let snow = precip_plan(&c, 3000).unwrap();
        assert!(snow.fall_speed < heavy.fall_speed / 3.0);
    }

    #[test]
    fn wind_slants_the_rain_in_the_direction_it_blows() {
        let mut c = WeatherConditions {
            precip: PrecipKind::Rain,
            precip_rate: 0.5,
            wind_speed: 6.0,
            // From the east (90°): the rain is carried to the west (-x).
            wind_from_deg: 90.0,
            ..WeatherConditions::default()
        };
        let p = precip_plan(&c, 1000).unwrap();
        assert!(p.drift.x < -3.0 && p.drift.y.abs() < 1e-3, "{:?}", p.drift);
        // From the north (0°): carried south (+z).
        c.wind_from_deg = 0.0;
        assert!(precip_plan(&c, 1000).unwrap().drift.y > 3.0);
        // A storm cannot make the rain fly sideways.
        c.wind_speed = 40.0;
        let p = precip_plan(&c, 1000).unwrap();
        assert!(p.drift.length() <= p.fall_speed * 1.5 + 1e-4);
    }

    #[test]
    fn the_look_is_scene_linear_and_follows_the_time_of_day() {
        let fog = WeatherConditions {
            fog_visibility_m: 100.0,
            ..WeatherConditions::default()
        };
        let day = weather_look(&fog, [0.6, 0.65, 0.75], 45.0);
        let night = weather_look(&fog, [0.02, 0.02, 0.04], -30.0);
        // sRGB 0.6 is about 0.32 linear: darker than the sRGB number.
        assert!(day.fog[0] < 0.5 && day.fog[0] > 0.2, "{:?}", day.fog);
        assert!(night.fog[0] < 0.01 && night.cloud < 0.06 && night.precip < 0.4);
        assert!((day.cloud - 1.0 + 0.15 * fogginess(&fog)).abs() < 1e-4);
    }

    #[test]
    fn clouds_drift_downwind_and_dim_at_night() {
        let mut c = WeatherConditions {
            wind_from_deg: 90.0,
            wind_speed: 2.0,
            ..WeatherConditions::default()
        };
        let d = cloud_drift(&c);
        assert!((d.x + 6.0).abs() < 1e-4 && d.y.abs() < 1e-4);
        assert!(cloud_tint(&c, -30.0) < 0.06);
        assert!((cloud_tint(&c, 45.0) - 1.0).abs() < 1e-4);
        c.precip_rate = 1.0;
        assert!(cloud_tint(&c, 45.0) < 0.75);
    }

    #[test]
    fn surfaces_get_wet_and_snowy_from_the_flags() {
        let dry = surface_state(&WeatherConditions::default());
        assert_eq!(dry, SurfaceState::default());
        let w = WeatherFile {
            ground_wet: [78.0, 255.0, 115.9],
            ..WeatherFile::default()
        };
        let wet = surface_state(&WeatherConditions::from_file(&w, &clouds()));
        assert!((wet.wet - 1.0).abs() < 1e-6);
        assert!(wet.puddles > 0.4 && wet.puddles <= 0.45);
        let s = WeatherFile {
            ground_wet: [1.7, 255.0, 0.0],
            snow: true,
            snow_on_road: true,
            ..WeatherFile::default()
        };
        let snow = surface_state(&WeatherConditions::from_file(&s, &clouds()));
        assert!(snow.wet < 0.5 && snow.snow_ground > 0.8 && snow.snow_road > 0.6);
        // Only the `[snow]` flag covers the ground.
        let only_road = WeatherFile {
            snow_on_road: true,
            ..WeatherFile::default()
        };
        let o = surface_state(&WeatherConditions::from_file(&only_road, &clouds()));
        assert_eq!((o.snow_ground, o.snow_road > 0.0), (0.0, true));
    }

    #[test]
    fn road_material_darkens_when_wet_and_whitens_under_snow() {
        let base = [0.4, 0.4, 0.4];
        let (c, r) = road_surface(base, 0.9, &SurfaceState::default());
        assert_eq!((c, r), (base, 0.9));
        let wet = SurfaceState {
            wet: 1.0,
            ..SurfaceState::default()
        };
        let (c, r) = road_surface(base, 0.9, &wet);
        assert!(c[0] < 0.4 && r < 0.5);
        let snow = SurfaceState {
            snow_road: 0.7,
            ..SurfaceState::default()
        };
        assert!(road_surface(base, 0.9, &snow).0[0] > 0.7);
    }

    #[test]
    fn precipitation_quality_keys_and_budgets() {
        for q in [
            PrecipitationQuality::Off,
            PrecipitationQuality::Low,
            PrecipitationQuality::Medium,
            PrecipitationQuality::High,
        ] {
            assert_eq!(PrecipitationQuality::parse(q.key()), Some(q));
        }
        assert_eq!(PrecipitationQuality::Off.budget(), 0);
        assert!(PrecipitationQuality::High.budget() > PrecipitationQuality::Low.budget());
        assert_eq!(PrecipitationQuality::parse("ultra"), None);
        assert_eq!(
            WeatherEffects::default().precipitation,
            PrecipitationQuality::Medium
        );
    }

    #[test]
    fn weather_kinds_from_preset_words() {
        assert_eq!(WeatherKind::of_preset("Rainy Afternoon"), WeatherKind::Rain);
        assert_eq!(WeatherKind::of_preset("Sommerlich"), WeatherKind::Clear);
        assert!(WeatherKind::Fog.overcast() && !WeatherKind::Clear.overcast());
    }

    fn ground_fog_conditions(visibility: f32) -> WeatherConditions {
        WeatherConditions {
            fog_visibility_m: visibility,
            fog_density: 1.0,
            ..WeatherConditions::default()
        }
    }

    #[test]
    fn only_low_visibility_presets_get_a_fog_layer() {
        assert_eq!(
            fog_layer_thickness(&ground_fog_conditions(75.0)),
            Some(30.0)
        );
        assert_eq!(
            fog_layer_thickness(&ground_fog_conditions(FOG_LAYER_MAX_VISIBILITY_M)),
            Some(60.0)
        );
        for v in [500.0, 700.0, 6000.0, 50_000.0] {
            assert_eq!(fog_layer_thickness(&ground_fog_conditions(v)), None, "{v}");
        }
        // Very thin fog still has a layer of at least 25 m; a thin factor makes it thinner.
        assert_eq!(
            fog_layer_thickness(&ground_fog_conditions(20.0)),
            Some(25.0)
        );
        let mut c = ground_fog_conditions(150.0);
        c.fog_density = 0.5;
        assert_eq!(fog_layer_thickness(&c), Some(45.0));
    }

    #[test]
    fn inside_the_layer_the_fog_is_the_presets_own() {
        for h in [0.0, 2.0, 15.0] {
            let p = ground_fog(75.0, 30.0, h, 0.0);
            assert_eq!((p.visibility_m, p.sky_clear), (75.0, 0.0), "h={h}");
            let looking_up = ground_fog(75.0, 30.0, h, 40.0);
            assert_eq!(looking_up.visibility_m, 75.0, "h={h}");
        }
    }

    #[test]
    fn the_fog_lifts_smoothly_and_monotonically_with_height() {
        let mut last = ground_fog(75.0, 30.0, 0.0, -10.0);
        let mut h = 0.0;
        while h <= 400.0 {
            h += 0.5;
            let p = ground_fog(75.0, 30.0, h, -10.0);
            assert!(p.visibility_m >= last.visibility_m - 1e-3, "h={h}");
            assert!(p.sky_clear >= last.sky_clear - 1e-6, "h={h}");
            // No popping: a half metre step moves the visibility by a few metres at most.
            assert!(p.visibility_m - last.visibility_m < 3.0, "h={h}");
            last = p;
        }
        let at = |h: f32| ground_fog(75.0, 30.0, h, 0.0);
        let (a, b, c, d) = (at(2.0), at(20.0), at(40.0), at(70.0));
        assert!(a.visibility_m < b.visibility_m && b.visibility_m < c.visibility_m);
        assert!(c.visibility_m < d.visibility_m);
        assert!(d.sky_clear > 0.99 && a.sky_clear == 0.0);
        // From 2·T up the visibility is the layer's share of the path: V · h / T.
        assert!((d.visibility_m - 75.0 * 70.0 / 30.0).abs() < 0.01);
    }

    #[test]
    fn looking_up_from_above_the_layer_reaches_farther_but_not_from_inside() {
        let down = ground_fog(75.0, 30.0, 70.0, -30.0).visibility_m;
        let level = ground_fog(75.0, 30.0, 70.0, 0.0).visibility_m;
        let up = ground_fog(75.0, 30.0, 70.0, 15.0).visibility_m;
        assert_eq!(down, level);
        assert!((up - level * 4.0).abs() < 0.01, "{up} {level}");
        assert!(ground_fog(75.0, 30.0, 70.0, 7.0).visibility_m > level);
        // Never beyond the distance where the fog is off.
        assert!(ground_fog(75.0, 30.0, 5000.0, 20.0).visibility_m < FOG_MAX_VISIBILITY_M);
    }

    #[test]
    fn the_view_follows_the_layer_rule_and_leaves_other_presets_alone() {
        let c = ground_fog_conditions(75.0);
        let low = ground_fog_view(&c, true, 2.0, 0.0);
        let high = ground_fog_view(&c, true, 70.0, 0.0);
        assert_eq!(low.layer_m, Some(30.0));
        assert_eq!(low.effective_visibility_m, 75.0);
        assert!(high.effective_visibility_m > 170.0 && high.sky_clear > 0.99);
        for v in [500.0, 700.0] {
            let c = ground_fog_conditions(v);
            let view = ground_fog_view(&c, true, 70.0, 0.0);
            assert_eq!(view.layer_m, None);
            assert_eq!((view.effective_visibility_m, view.sky_clear), (v, 0.0));
        }
        assert_eq!(ground_fog_view(&c, false, 70.0, 0.0).layer_m, None);
    }

    #[test]
    fn a_cleared_sky_keeps_only_a_hazy_rim_and_the_overcast() {
        let c = ground_fog_conditions(75.0);
        assert_eq!(sky_veil_clear(&c, 0.5, 0.0), sky_veil(&c, 0.5));
        let zenith = sky_veil_clear(&c, 0.0, 1.0);
        let mid = sky_veil_clear(&c, 0.6, 1.0);
        let rim = sky_veil_clear(&c, 1.0, 1.0);
        assert!(zenith == 0.0 && mid == 0.0 && rim > 0.3 && rim < sky_veil(&c, 1.0));
        // The frame's top third of a horizon view (4° … 11° up) keeps only a faint veil.
        let band: Vec<f32> = (0..=14)
            .map(|i| sky_veil_clear(&c, 1.0 - (4.0 + i as f32 * 0.5) / 90.0, 1.0))
            .collect();
        let mean = band.iter().sum::<f32>() / band.len() as f32;
        assert!(band[0] < 0.12 && mean < 0.05, "{band:?} {mean}");
        let mut overcast = c.clone();
        overcast.cloud = Some(CloudSpec {
            name: "Overcast 1".into(),
            texture: None,
            size_m: 1000.0,
            base_m: 100.0,
            cover: CloudCover::Overcast,
            coverage: 1.0,
        });
        assert!(sky_veil_clear(&overcast, 0.0, 1.0) > 0.1);
    }
}
