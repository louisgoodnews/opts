//! Unit tests of `weather.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::catalogue::{WeatherCatalogue, BUILTIN_IDS};
use super::fog::{
    cloud_tint, effective_cover, fog_color, fog_density, fog_layer_thickness, fogginess,
    ground_fog, ground_fog_view, light_scales, sky_veil, sky_veil_clear,
    FOG_LAYER_MAX_VISIBILITY_M, FOG_MAX_SATURATION, FOG_MAX_VISIBILITY_M,
};
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

mod catalogue_tests;
mod conditions_tests;

mod fog_tests;

mod look_tests;

