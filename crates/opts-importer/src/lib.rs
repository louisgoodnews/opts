//! `opts-importer`: reads legacy OMSI content with `opts-omsi` and produces the native model of
//! `opts-model` ([ADR-0014](../../docs/src/adr/0014-data-model-separation.md)). It is the one
//! crate allowed to know both; the engine crates ask the model's `ContentSource` and the game (the
//! composition root) plugs an OMSI-backed source in.
//!
//! Skeleton: one vehicle (`.bus`) with its names, mass and axles.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use opts_model::{AxleDefinition, VehicleDefinition};
use opts_omsi::formats::vehicles::BusManifest;

/// Why a vehicle could not be imported.
#[derive(Debug)]
pub struct ImportError(pub String);

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot import the vehicle: {}", self.0)
    }
}

impl std::error::Error for ImportError {}

/// The native definition of a parsed `.bus` file.
#[must_use]
pub fn vehicle_from_manifest(bus: &BusManifest) -> VehicleDefinition {
    let (manufacturer, model, livery) = bus
        .friendly_name
        .as_ref()
        .map_or_else(Default::default, |f| {
            (f.manufacturer.clone(), f.model.clone(), f.livery.clone())
        });
    VehicleDefinition {
        manufacturer,
        model,
        livery,
        mass_tonnes: bus.mass_tonnes,
        axles: bus
            .axles
            .iter()
            .map(|a| AxleDefinition {
                longitudinal_m: a.longitudinal_m,
                wheel_diameter_m: a.wheel_diameter_m,
            })
            .collect(),
        ..Default::default()
    }
}

/// Imports a vehicle from the bytes of its `.bus` file.
///
/// # Errors
/// The file is not a readable `.bus`.
pub fn import_vehicle(bus_bytes: &[u8]) -> Result<VehicleDefinition, ImportError> {
    let bus = BusManifest::parse_bytes(bus_bytes).map_err(|e| ImportError(e.to_string()))?;
    Ok(vehicle_from_manifest(&bus))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUS: &str = "[friendlyname]\nMAN\nSD202\nBVG\n\n[mass]\n11.5\n\n[axle]\n2.3\n1.0\n0\n0\n0\n0\n0\n[axle]\n-3.1\n1.0\n0\n0\n0\n0\n0\n";

    #[test]
    fn a_bus_file_becomes_a_native_vehicle_that_round_trips_through_toml() {
        let v = import_vehicle(BUS.as_bytes()).unwrap();
        assert_eq!(
            (v.manufacturer.as_str(), v.model.as_str()),
            ("MAN", "SD202")
        );
        let back = VehicleDefinition::from_toml(&v.to_toml()).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn a_vehicle_without_a_name_imports_with_empty_names() {
        let v = import_vehicle(b"[mass]\n9.0\n").unwrap();
        assert!(v.model.is_empty());
    }
}
