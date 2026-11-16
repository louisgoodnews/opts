//! Store tests: the schema, atomic writes, unique ids, damaged files.

use super::*;

fn dir(name: &str) -> PathBuf {
    let d = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(".local/share/opts/benchmark/formation/store-tests")
        .join(format!("{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

fn sample() -> TrainFormation {
    TrainFormation {
        name: "Three NF6D".into(),
        segments: vec![
            FormationSegment {
                vehicle: VehicleRef::File("Vehicles\\Tram_NF6D\\NF6D_A.bus".into()),
                orientation: Orientation::Forward,
                powered: None,
            },
            FormationSegment {
                vehicle: VehicleRef::File("Vehicles\\Bm238\\By438_mi.ovh".into()),
                orientation: Orientation::Backward,
                powered: Some(false),
            },
            FormationSegment {
                vehicle: VehicleRef::Group("M-Train Cars".into()),
                orientation: Orientation::Forward,
                powered: Some(true),
            },
        ],
        source: FormationSource::ZugFile("Trains\\X10_112_IC-8.zug".into()),
    }
}

mod files;
mod schema;
