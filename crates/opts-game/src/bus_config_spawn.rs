//! Bus configurations in a running session (M-0077): the spawn request a configuration stands
//! for, and the configuration of the active session vehicle.

use std::fmt;
use std::path::PathBuf;

use crate::bus_config::{BusConfiguration, Checked};
use crate::new_game_screen::types::GameCatalogue;
#[cfg(test)]
use crate::session_vehicles::SessionVehicles;
use crate::session_vehicles::{EntrySel, SessionVehicleRecord, SpawnParams};

/// Why a configuration cannot be spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnProblem {
    /// The check found something that makes it unusable (the first fatal problem).
    Unusable(String),
    /// The vehicle is in the catalogue but its `.bus` file is not found in the installation.
    BusFileMissing(String),
}

impl fmt::Display for SpawnProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unusable(why) => f.write_str(why),
            Self::BusFileMissing(file) => write!(f, "the file {file} was not found"),
        }
    }
}

/// The spawn request of a checked configuration at `entry`; `find_bus` resolves a catalogue
/// `.bus` name to a file of the installation.
///
/// # Errors
/// [`SpawnProblem`] when the configuration is unusable or its `.bus` file is gone.
pub fn spawn_params(
    config: &BusConfiguration,
    checked: &Checked,
    catalogue: &GameCatalogue,
    find_bus: impl Fn(&str) -> Option<PathBuf>,
    entry: EntrySel,
) -> Result<SpawnParams, SpawnProblem> {
    if let Some(fatal) = checked.problems.iter().find(|p| p.is_fatal()) {
        return Err(SpawnProblem::Unusable(fatal.to_string()));
    }
    let vehicle = catalogue
        .vehicles
        .iter()
        .find(|v| v.id == config.vehicle)
        .ok_or_else(|| SpawnProblem::Unusable(format!("{} is not installed", config.vehicle)))?;
    let file = find_bus(&vehicle.bus_file)
        .ok_or_else(|| SpawnProblem::BusFileMissing(vehicle.bus_file.clone()))?;
    Ok(SpawnParams {
        catalogue_id: vehicle.id.clone(),
        name: vehicle.full_name.clone(),
        bus_file: file.display().to_string(),
        repaint: checked.repaint.clone(),
        setvars: checked.setvars.clone(),
        entry,
        formation: None,
    })
}

/// The configuration `name` of a session vehicle (its catalogue id, repaint and equipment
/// options); `None` for a vehicle that came from a bare path (no catalogue id).
#[must_use]
pub fn from_record(
    name: &str,
    record: &SessionVehicleRecord,
    now: i64,
) -> Option<BusConfiguration> {
    if record.catalogue_id.is_empty() {
        return None;
    }
    Some(BusConfiguration::new(
        name,
        &record.catalogue_id,
        &record.repaint,
        record.setvars.clone(),
        now,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus_config::Problem;
    use crate::new_game_screen::types::VehicleItem;

    fn catalogue() -> GameCatalogue {
        GameCatalogue {
            maps: Vec::new(),
            vehicles: vec![{
                let mut v = VehicleItem::named("c2", "Solo", "C2");
                v.bus_file = "Vehicles/C2/c2.bus".into();
                v
            }],
        }
    }

    fn config() -> BusConfiguration {
        BusConfiguration::new("Morning", "c2", "Red", vec![("Heater".into(), 1)], 0)
    }

    fn checked(problems: Vec<Problem>) -> Checked {
        Checked {
            repaint: "Red".into(),
            setvars: vec![("Heater".into(), 1)],
            problems,
        }
    }

    #[test]
    fn a_usable_configuration_becomes_a_spawn_request() {
        let p = spawn_params(
            &config(),
            &checked(vec![]),
            &catalogue(),
            |f| Some(PathBuf::from(format!("/omsi/{f}"))),
            EntrySel::Id(3),
        )
        .expect("spawn");
        assert_eq!(p.catalogue_id, "c2");
        assert_eq!(p.name, catalogue().vehicles[0].full_name);
        assert_eq!(p.bus_file, "/omsi/Vehicles/C2/c2.bus");
        assert_eq!(p.repaint, "Red");
        assert_eq!(p.setvars, vec![("Heater".to_owned(), 1)]);
        assert_eq!(p.entry, EntrySel::Id(3));
    }

    #[test]
    fn a_dropped_repaint_still_spawns_with_the_default() {
        let mut ch = checked(vec![Problem::RepaintMissing {
            repaint: "Red".into(),
        }]);
        ch.repaint.clear();
        let p = spawn_params(
            &config(),
            &ch,
            &catalogue(),
            |_| Some("/x".into()),
            EntrySel::Default,
        )
        .expect("spawn");
        assert!(p.repaint.is_empty());
    }

    #[test]
    fn a_missing_vehicle_or_file_is_refused_with_a_reason() {
        let gone = checked(vec![Problem::VehicleMissing {
            vehicle: "c2".into(),
        }]);
        let e = spawn_params(
            &config(),
            &gone,
            &catalogue(),
            |_| Some("/x".into()),
            EntrySel::Default,
        )
        .unwrap_err();
        assert!(matches!(e, SpawnProblem::Unusable(_)));
        let e = spawn_params(
            &config(),
            &checked(vec![]),
            &catalogue(),
            |_| None,
            EntrySel::Default,
        )
        .unwrap_err();
        assert_eq!(e, SpawnProblem::BusFileMissing("Vehicles/C2/c2.bus".into()));
    }

    #[test]
    fn a_bare_path_vehicle_has_no_configuration() {
        let mut registry = SessionVehicles::default();
        let id = registry.request_spawn(SpawnParams {
            catalogue_id: String::new(),
            name: "Bare".into(),
            bus_file: "/x.bus".into(),
            repaint: String::new(),
            setvars: Vec::new(),
            entry: EntrySel::Default,
            formation: None,
        });
        let mut rec = registry
            .records()
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .expect("record");
        assert!(from_record("x", &rec, 0).is_none());
        rec.catalogue_id = "c2".into();
        rec.repaint = "Red".into();
        let c = from_record("x", &rec, 7).expect("config");
        assert_eq!(
            (c.vehicle.as_str(), c.repaint.as_str(), c.created),
            ("c2", "Red", 7)
        );
    }
}
