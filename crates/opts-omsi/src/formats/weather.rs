//! Weather files: presets (`Weather/*.owt`, scenario `*.osn.owt`), the cloud catalogue
//! (`Weather/clouds.cfg`) and the localised names and descriptions (`<Name>_<LANG>.dsc`).
//!
//! Layout and evidence: `docs/src/formats/text/environment.md`; use in the game:
//! `docs/src/architecture/weather.md`. The encoding differs per file (CP1252, UTF-8, UTF-16LE), so
//! every file goes through [`decode_bytes`].

use std::collections::BTreeMap;
use std::path::Path;

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

/// How a cloud layer is drawn (`clouds.cfg`, last line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudCover {
    /// `sct`: scattered clouds with gaps.
    Scattered,
    /// `ovc`: a closed overcast layer.
    Overcast,
}

/// One `[cloudtype]` of `clouds.cfg`.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudType {
    /// Name (`Cumulus 1`, `Overcast 1`), matched case-insensitively by `[clouds]`.
    pub name: String,
    /// Texture file below `Texture/`.
    pub texture: String,
    /// Edge length of one texture repeat in metres.
    pub size_m: f32,
    /// Cover class.
    pub cover: CloudCover,
}

/// The cloud catalogue (`Weather/clouds.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CloudTypes(pub Vec<CloudType>);

impl CloudTypes {
    /// Parses `clouds.cfg`: `[cloudtype]` blocks of four lines (name, texture, size in metres,
    /// `sct` or `ovc`); the German explanation at the top is skipped. A block with a bad number
    /// or class is dropped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let blocks = scan_blocks_with(&decode_bytes(bytes), |k| (k == "cloudtype").then_some(4));
        Self(
            blocks
                .iter()
                .filter(|b| b.keyword == "cloudtype")
                .filter_map(|b| {
                    let p = &b.parameters;
                    let cover = match p.get(3)?.trim().to_ascii_lowercase().as_str() {
                        "sct" => CloudCover::Scattered,
                        "ovc" => CloudCover::Overcast,
                        _ => return None,
                    };
                    Some(CloudType {
                        name: p.first()?.trim().to_owned(),
                        texture: p.get(1)?.trim().to_owned(),
                        size_m: number(p, 2).filter(|s| *s > 0.0)?,
                        cover,
                    })
                })
                .collect(),
        )
    }

    /// The type with this name (case-insensitive).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&CloudType> {
        self.0
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name.trim()))
    }
}

/// Name and description of a `.dsc` file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeatherText {
    /// `[name]`.
    pub name: String,
    /// `[description]` up to `[end]`.
    pub description: String,
}

impl WeatherText {
    /// Parses a `<Name>_<LANG>.dsc`; `None` when it has no name.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let blocks = scan_blocks_with(&decode_bytes(bytes), arity);
        let find = |kw: &str| blocks.iter().find(|b| b.keyword == kw);
        let name = find("name")?.parameters.first()?.trim().to_owned();
        if name.is_empty() {
            return None;
        }
        Some(Self {
            name,
            description: find("description")
                .map(|b| b.parameters.join("\n"))
                .unwrap_or_default(),
        })
    }
}

/// The OMSI language code (`DEU`) of a UI locale tag (`de`, `de-DE`), `None` for languages OMSI
/// does not ship descriptions in.
#[must_use]
pub fn omsi_language(locale: &str) -> Option<&'static str> {
    match locale
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "de" => Some("DEU"),
        "en" => Some("ENG"),
        "fr" => Some("FRA"),
        "hu" => Some("HUN"),
        "pl" => Some("POL"),
        _ => None,
    }
}

/// A preset as found on disk: the file and its texts by OMSI language code.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetFile {
    /// Stable id: the file stem (`Sommerlich`, `#CAVOK`).
    pub id: String,
    /// The parsed `.owt`.
    pub weather: WeatherFile,
    /// `.dsc` texts by language code (`DEU`, `ENG`, …).
    pub texts: BTreeMap<String, WeatherText>,
}

impl PresetFile {
    /// Name for a language code: its `.dsc`, else English, else the `.owt`'s own name, else the
    /// id.
    #[must_use]
    pub fn name(&self, language: Option<&str>) -> String {
        self.text(language).map_or_else(
            || {
                let n = self.weather.clean_name();
                if n.is_empty() {
                    self.id.clone()
                } else {
                    n.to_owned()
                }
            },
            |t| strip_brackets(&t.name).to_owned(),
        )
    }

    /// Description for a language code, with the same fallbacks as [`Self::name`].
    #[must_use]
    pub fn description(&self, language: Option<&str>) -> String {
        self.text(language).map_or_else(
            || self.weather.description.clone(),
            |t| t.description.clone(),
        )
    }

    fn text(&self, language: Option<&str>) -> Option<&WeatherText> {
        language
            .and_then(|l| self.texts.get(l))
            .or_else(|| self.texts.get("ENG"))
    }
}

/// The contents of an install's `Weather` folder.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeatherFolder {
    /// Presets sorted by id (`#CAVOK` first).
    pub presets: Vec<PresetFile>,
    /// The cloud catalogue (empty when `clouds.cfg` is missing).
    pub clouds: CloudTypes,
}

impl WeatherFolder {
    /// Reads `dir` (the `Weather` folder): every `*.owt` that parses (a broken one is skipped)
    /// with its `*_<LANG>.dsc` files, and `clouds.cfg`. File names are matched without regard to
    /// case.
    ///
    /// # Errors
    /// Returns the I/O error message when the folder cannot be listed.
    pub fn read(dir: &Path) -> Result<Self, String> {
        let entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
            .filter(|e| e.path().is_file())
            .collect();
        let name_of = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().into_owned();
        let mut folder = Self::default();
        for e in &entries {
            let file = name_of(e);
            let lower = file.to_ascii_lowercase();
            if lower == "clouds.cfg" {
                if let Ok(bytes) = std::fs::read(e.path()) {
                    folder.clouds = CloudTypes::parse(&bytes);
                }
            } else if let Some(id) = lower
                .ends_with(".owt")
                .then(|| file[..file.len() - 4].to_owned())
            {
                let Ok(weather) = std::fs::read(e.path())
                    .map_err(|e| e.to_string())
                    .and_then(|b| WeatherFile::parse(&b))
                else {
                    continue;
                };
                folder.presets.push(PresetFile {
                    id,
                    weather,
                    texts: BTreeMap::new(),
                });
            }
        }
        for e in &entries {
            let file = name_of(e);
            if !file.to_ascii_lowercase().ends_with(".dsc") {
                continue;
            }
            let stem = &file[..file.len() - 4];
            let Some((id, lang)) = stem.rsplit_once('_') else {
                continue;
            };
            let Some(preset) = folder
                .presets
                .iter_mut()
                .find(|p| p.id.eq_ignore_ascii_case(id))
            else {
                continue;
            };
            if let Some(text) = std::fs::read(e.path())
                .ok()
                .and_then(|b| WeatherText::parse(&b))
            {
                preset.texts.insert(lang.to_ascii_uppercase(), text);
            }
        }
        folder.presets.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(folder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORM: &str = "[name]\nSturm\n\n[description]\nEin Sturm.\nZweite Zeile.\n[end]\n\n[fog]\n700.0\n0.5\n\n[wind]\n209\n18.9\n\n[temp]\n9\n9\n\n[press]\n1013\n\n[clouds]\nCumulus 3\n200\n\n[precip]\n1\n132\n0\n-2\n0\n\n[groundwet]\n30.6\n255\n0\n";

    #[test]
    fn parses_every_block() {
        let w = WeatherFile::parse(STORM.as_bytes()).unwrap();
        assert_eq!(w.name, "Sturm");
        assert_eq!(w.description, "Ein Sturm.\nZweite Zeile.");
        assert_eq!(w.fog.visibility_m, 700.0);
        assert_eq!(w.fog.density, 0.5);
        assert_eq!(w.wind.speed, 18.9);
        assert_eq!(w.temp.dew_point_c, 9.0);
        assert_eq!(w.pressure_hpa, 1013.0);
        let c = w.clouds.as_ref().unwrap();
        assert_eq!((c.kind.as_str(), c.base_m), ("Cumulus 3", 200.0));
        assert_eq!(w.precip.kind, PrecipKind::Rain);
        assert_eq!(w.precip.intensity, 132.0);
        assert_eq!(w.precip.extra, [0.0, -2.0, 0.0]);
        assert_eq!(w.ground_wet, [30.6, 255.0, 0.0]);
        assert!(!w.snow && !w.snow_on_road);
    }

    #[test]
    fn no_clouds_flags_and_decimal_commas() {
        let text = "[fog]\n75,5\n1\n[clouds]\n-1\n50\n[snow]\n\n[snowOnRoad]\n";
        let w = WeatherFile::parse(text.as_bytes()).unwrap();
        assert_eq!(w.fog.visibility_m, 75.5);
        assert!(w.clouds.is_none());
        assert!(w.snow && w.snow_on_road);
    }

    #[test]
    fn explanatory_text_between_blocks_is_not_data() {
        let text = "=====\n\tOmsi Weather File\n=====\n[name]\n<Bestes Wetter>\n\n[description]\nKlar.\n[end]\n\n\t[fog]\n\t{range}\n\t{height}\n\nText ohne Marker.\n\n[fog]\n50000\n1\n\n[precip]\n0\n32\n";
        let w = WeatherFile::parse(text.as_bytes()).unwrap();
        assert_eq!(w.clean_name(), "Bestes Wetter");
        assert_eq!(w.description, "Klar.");
        assert_eq!(w.fog.visibility_m, 50_000.0);
        // A short `[precip]` (two lines) keeps the rest at zero.
        assert_eq!(w.precip.kind, PrecipKind::None);
        assert_eq!(w.precip.intensity, 32.0);
        assert_eq!(w.precip.extra, [0.0; 3]);
    }

    #[test]
    fn utf16_and_cp1252_files_decode_alike() {
        let text = "[name]\nEiseskälte\n[temp]\n-16\n-24.9\n";
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let cp1252: Vec<u8> = text.chars().map(|c| c as u32 as u8).collect();
        let a = WeatherFile::parse(&utf16).unwrap();
        let b = WeatherFile::parse(&cp1252).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.name, "Eiseskälte");
        assert_eq!(a.temp.celsius, -16.0);
    }

    #[test]
    fn a_file_without_keywords_is_an_error() {
        assert!(WeatherFile::parse(b"hello\nworld\n").is_err());
        assert!(WeatherFile::parse(b"").is_err());
    }

    #[test]
    fn cloud_catalogue_skips_the_explanation() {
        let text = "Hier werden alle Wolkentypen definiert:\n\n\t[cloudtype]\n\tname\n\ttexture\n\n[cloudtype]\nCumulus 1\nCumulus_1.tga\n2000\nsct\n\n[cloudtype]\nOvercast 1\nOVC_1.bmp\n1000\novc";
        let c = CloudTypes::parse(text.as_bytes());
        assert_eq!(c.0.len(), 2);
        let t = c.find("cumulus 1").unwrap();
        assert_eq!((t.texture.as_str(), t.size_m), ("Cumulus_1.tga", 2000.0));
        assert_eq!(t.cover, CloudCover::Scattered);
        assert_eq!(c.find("Overcast 1").unwrap().cover, CloudCover::Overcast);
        assert!(c.find("Cirrus").is_none());
    }

    #[test]
    fn descriptions_and_language_codes() {
        let t =
            WeatherText::parse(b"\n[name]\n<Best Weather>\n\n[description]\nClear sky.\n[end]\n")
                .unwrap();
        assert_eq!(strip_brackets(&t.name), "Best Weather");
        assert_eq!(t.description, "Clear sky.");
        assert!(WeatherText::parse(b"[description]\nx\n[end]").is_none());
        assert_eq!(omsi_language("de"), Some("DEU"));
        assert_eq!(omsi_language("en-GB"), Some("ENG"));
        assert_eq!(omsi_language("ja"), None);
    }

    #[test]
    fn preset_names_fall_back_to_english_then_the_file() {
        let mut texts = BTreeMap::new();
        texts.insert(
            "ENG".to_owned(),
            WeatherText {
                name: "Storm".into(),
                description: "A storm.".into(),
            },
        );
        let weather = WeatherFile {
            name: "Sturm".into(),
            ..WeatherFile::default()
        };
        let p = PresetFile {
            id: "Sturm".into(),
            weather: weather.clone(),
            texts,
        };
        assert_eq!(p.name(Some("DEU")), "Storm");
        assert_eq!(p.description(None), "A storm.");
        let bare = PresetFile {
            id: "Sturm".into(),
            weather,
            texts: BTreeMap::new(),
        };
        assert_eq!(bare.name(Some("DEU")), "Sturm");
    }

    #[test]
    fn precip_codes() {
        assert_eq!(PrecipKind::from_code(0.0), PrecipKind::None);
        assert_eq!(PrecipKind::from_code(0.999_999_9), PrecipKind::Rain);
        assert_eq!(PrecipKind::from_code(2.0), PrecipKind::Snow);
        assert_eq!(PrecipKind::from_code(7.0), PrecipKind::None);
        assert_eq!(PrecipKind::Snow.code(), 2);
    }
}
