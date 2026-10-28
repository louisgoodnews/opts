//! Weather files: presets (`Weather/*.owt`, scenario `*.osn.owt`), the cloud catalogue
//! (`Weather/clouds.cfg`) and the localised names and descriptions (`<Name>_<LANG>.dsc`).
//!
//! Layout and evidence: `docs/src/formats/text/environment.md`; use in the game:
//! `docs/src/architecture/weather.md`. The encoding differs per file (CP1252, UTF-8, UTF-16LE), so
//! every file goes through [`decode_bytes`].

use crate::formats::text::{decode_bytes, parse_float, scan_blocks_with, OmsiBlock};

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
    /// `[name]` (often `<Angle Brackets>` in the shipped files; see [`strip_brackets`]).
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

/// Removes one pair of `<…>` around a name (`<Best Weather>` → `Best Weather`).
#[must_use]
pub fn strip_brackets(name: &str) -> &str {
    let n = name.trim();
    n.strip_prefix('<')
        .and_then(|n| n.strip_suffix('>'))
        .unwrap_or(n)
        .trim()
}

/// Parameter counts of the keywords, so the explanatory text between blocks is not taken for
/// values (`#CAVOK.owt` carries a long German comment between its blocks).
fn arity(keyword: &str) -> Option<usize> {
    match keyword {
        "name" | "press" => Some(1),
        "fog" | "wind" | "temp" | "clouds" => Some(2),
        "precip" => Some(5),
        "groundwet" => Some(3),
        "snow" | "snowonroad" | "end" => Some(0),
        _ => None,
    }
}

fn number(params: &[String], i: usize) -> Option<f32> {
    parse_float(params.get(i)?).ok().filter(|v| v.is_finite())
}

fn pair(block: &OmsiBlock) -> Option<(f32, f32)> {
    Some((number(&block.parameters, 0)?, number(&block.parameters, 1)?))
}

impl WeatherFile {
    /// Parses a `.owt` (or a scenario's `.osn.owt`). Blocks that are missing or malformed keep
    /// the defaults of [`WeatherFile::default`]; a file without any known block is an error.
    ///
    /// # Errors
    /// Returns a message when the text holds none of the weather keywords.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let blocks = scan_blocks_with(&decode_bytes(bytes), arity);
        let mut w = Self::default();
        let mut known = 0;
        for b in &blocks {
            let p = &b.parameters;
            match b.keyword.as_str() {
                "name" => {
                    w.name = p.first().cloned().unwrap_or_default();
                }
                "description" => w.description = p.join("\n"),
                "fog" => {
                    if let Some((v, d)) = pair(b) {
                        w.fog = Fog {
                            visibility_m: v,
                            density: d,
                        };
                    }
                }
                "wind" => {
                    if let Some((d, s)) = pair(b) {
                        w.wind = Wind {
                            direction_deg: d,
                            speed: s,
                        };
                    }
                }
                "temp" => {
                    if let Some((t, d)) = pair(b) {
                        w.temp = Temperature {
                            celsius: t,
                            dew_point_c: d,
                        };
                    }
                }
                "press" => {
                    if let Some(v) = number(p, 0) {
                        w.pressure_hpa = v;
                    }
                }
                "clouds" => {
                    // A number as the type (`-1`) means no clouds.
                    let kind = p.first().map(|s| s.trim()).unwrap_or_default();
                    w.clouds =
                        (!kind.is_empty() && parse_float(kind).is_err()).then(|| CloudLayer {
                            kind: kind.to_owned(),
                            base_m: number(p, 1).unwrap_or(0.0),
                        });
                }
                "precip" => {
                    w.precip = Precip {
                        kind: PrecipKind::from_code(number(p, 0).unwrap_or(0.0)),
                        intensity: number(p, 1).unwrap_or(0.0),
                        extra: [
                            number(p, 2).unwrap_or(0.0),
                            number(p, 3).unwrap_or(0.0),
                            number(p, 4).unwrap_or(0.0),
                        ],
                    };
                }
                "groundwet" => {
                    for (i, v) in w.ground_wet.iter_mut().enumerate() {
                        *v = number(p, i).unwrap_or(0.0);
                    }
                }
                "snow" => w.snow = true,
                "snowonroad" => w.snow_on_road = true,
                "end" => {}
                _ => continue,
            }
            known += 1;
        }
        if known == 0 {
            return Err("no weather keyword found".into());
        }
        Ok(w)
    }

    /// The name without the angle brackets some files use.
    #[must_use]
    pub fn clean_name(&self) -> &str {
        strip_brackets(&self.name)
    }
}

mod folder;
pub use folder::*;

#[cfg(test)]
mod tests;
