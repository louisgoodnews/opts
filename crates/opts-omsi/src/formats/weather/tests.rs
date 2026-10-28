//! Tests of the `weather.cfg` parser (moved out of `mod.rs`, which stays under the 500-line limit).

use super::*;
use std::collections::BTreeMap;

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
    let t = WeatherText::parse(b"\n[name]\n<Best Weather>\n\n[description]\nClear sky.\n[end]\n")
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
