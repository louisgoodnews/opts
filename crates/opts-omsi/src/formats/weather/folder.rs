//! Cloud catalogue (`clouds.cfg`), localised `.dsc` descriptions, and `Weather/` folder scanner.

use std::collections::BTreeMap;
use std::path::Path;

use super::*;

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
