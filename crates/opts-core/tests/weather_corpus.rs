//! Corpus test: every installed weather preset (`Weather/*.owt` and the scenario `*.osn.owt` files
//! of the maps, `Situations` and `Tutorials`) parses, the values lie in the documented ranges and
//! each cloud type of a preset exists in `clouds.cfg`.
//! Skipped unless `OMSI_PATH` points at an installation.

use std::path::{Path, PathBuf};

use opts_core::formats::weather::{PrecipKind, WeatherFile, WeatherFolder};

fn owt_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            owt_files(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("owt")) {
            out.push(p);
        }
    }
}

fn check(name: &str, w: &WeatherFile) {
    assert!(
        (1.0..=100_000.0).contains(&w.fog.visibility_m),
        "{name}: visibility"
    );
    assert!((0.0..=1.0).contains(&w.fog.density), "{name}: fog density");
    assert!(
        (0.0..=360.0).contains(&w.wind.direction_deg),
        "{name}: wind dir"
    );
    assert!((0.0..=60.0).contains(&w.wind.speed), "{name}: wind speed");
    assert!((-60.0..=60.0).contains(&w.temp.celsius), "{name}: temp");
    assert!(
        w.temp.dew_point_c <= w.temp.celsius + 0.01,
        "{name}: dew point above temp"
    );
    assert!(
        (900.0..=1100.0).contains(&w.pressure_hpa),
        "{name}: pressure"
    );
    assert!(
        (0.0..=255.0).contains(&w.precip.intensity),
        "{name}: intensity"
    );
    assert!(
        w.ground_wet.iter().all(|v| (0.0..=300.0).contains(v)),
        "{name}: groundwet"
    );
    if let Some(c) = &w.clouds {
        assert!((0.0..=20_000.0).contains(&c.base_m), "{name}: cloud base");
    }
}

#[test]
fn every_installed_weather_file_parses_in_range() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut files = Vec::new();
    owt_files(&root, &mut files);
    if files.is_empty() {
        eprintln!("no .owt files; skipping corpus test");
        return;
    }
    let folder = WeatherFolder::read(&root.join("Weather")).expect("Weather folder");
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        let w = WeatherFile::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        check(&f.display().to_string(), &w);
        if let Some(c) = w
            .clouds
            .as_ref()
            .filter(|c| folder.clouds.find(&c.kind).is_none())
        {
            // Map add-ons name cloud types that `clouds.cfg` does not have (`Cumulus 5`); the
            // stock presets must not.
            assert!(
                !f.starts_with(root.join("Weather")),
                "{}: unknown cloud type {}",
                f.display(),
                c.kind
            );
            eprintln!("{}: unknown cloud type {}", f.display(), c.kind);
        }
    }
    // The nine presets of the stock install, each with a name in German and English.
    assert!(
        folder.presets.len() >= 9,
        "{} presets",
        folder.presets.len()
    );
    assert!(folder.presets.iter().any(|p| p.id == "#CAVOK"));
    for p in &folder.presets {
        assert!(!p.name(Some("DEU")).is_empty() && !p.name(Some("ENG")).is_empty());
    }
    assert!(folder.clouds.0.len() >= 4);
    let snow = folder.presets.iter().find(|p| p.id == "Starker Schneefall");
    if let Some(s) = snow {
        assert_eq!(s.weather.precip.kind, PrecipKind::Snow);
        assert!(s.weather.snow && s.weather.snow_on_road);
    }
    eprintln!("{} weather files parsed", files.len());
}
