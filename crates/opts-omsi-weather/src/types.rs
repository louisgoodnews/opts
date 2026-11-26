//! Value types of a weather file: precipitation kinds, fog, wind, temperature, clouds, and the
//! [`WeatherFile`] record with its defaults.

/// What falls from the sky (`[precip]`, first value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrecipKind {
    /// Dry.
    #[default]
    None,
    /// Rain (1).
    Rain,
    /// Snow (2).
    Snow,
}

impl PrecipKind {
    /// The kind of the file's number (anything unknown is dry).
    #[must_use]
    pub fn from_code(code: f32) -> Self {
        match code.round() as i32 {
            1 => Self::Rain,
            2 => Self::Snow,
            _ => Self::None,
        }
    }

    /// The number the file and `PrecipType` use: 0 none, 1 rain, 2 snow.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Rain => 1,
            Self::Snow => 2,
        }
    }
}

/// `[fog]`: visibility and a 0…1 thickness factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    /// Distance in metres at which an object just stands out against the fog.
    pub visibility_m: f32,
    /// Thickness of the fog layer (the file says "height"; 0.5 … 1 observed).
    pub density: f32,
}

/// `[wind]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wind {
    /// Direction the wind comes **from**, degrees clockwise from north.
    pub direction_deg: f32,
    /// Speed in m/s (by plausibility ❓).
    pub speed: f32,
}

/// `[temp]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature {
    /// Air temperature in °C.
    pub celsius: f32,
    /// Dew point in °C (equal to the temperature in fog).
    pub dew_point_c: f32,
}

/// `[clouds]`: a layer from `clouds.cfg`.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudLayer {
    /// Cloud type name as in `clouds.cfg` (`Cumulus 1`).
    pub kind: String,
    /// Base height in metres.
    pub base_m: f32,
}

/// `[precip]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Precip {
    /// Type.
    pub kind: PrecipKind,
    /// Second value: an intensity on a 0…255 scale for rain and snow (32 = drizzle).
    pub intensity: f32,
    /// Values 3–5, meaning unknown ❓ (`0 -2 0` in the corpus).
    pub extra: [f32; 3],
}

/// One weather preset (`.owt`).
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherFile {
    /// `[name]` (often `<Angle Brackets>` in the shipped files; see [`super::strip_brackets`]).
    pub name: String,
    /// `[description]` up to `[end]`, lines joined by `\n`.
    pub description: String,
    /// `[fog]`.
    pub fog: Fog,
    /// `[wind]`.
    pub wind: Wind,
    /// `[temp]`.
    pub temp: Temperature,
    /// `[press]` in hPa.
    pub pressure_hpa: f32,
    /// `[clouds]`; `None` for the type `-1`.
    pub clouds: Option<CloudLayer>,
    /// `[precip]`.
    pub precip: Precip,
    /// `[groundwet]`, three values of unknown meaning ❓ (`0 0 0` dry).
    pub ground_wet: [f32; 3],
    /// `[snow]` is present.
    pub snow: bool,
    /// `[snowOnRoad]` is present.
    pub snow_on_road: bool,
}

impl Default for WeatherFile {
    /// The standard conditions (`#CAVOK`): clear, no wind, 15 °C.
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            fog: Fog {
                visibility_m: 50_000.0,
                density: 1.0,
            },
            wind: Wind {
                direction_deg: 0.0,
                speed: 0.0,
            },
            temp: Temperature {
                celsius: 15.0,
                dew_point_c: 5.0,
            },
            pressure_hpa: 1013.0,
            clouds: None,
            precip: Precip::default(),
            ground_wet: [0.0; 3],
            snow: false,
            snow_on_road: false,
        }
    }
}
