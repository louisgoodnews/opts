//! The native vehicle definition (stub: the fields every vehicle has, enough for one round trip).

use serde::{Deserialize, Serialize};

/// Schema version written to every native vehicle file.
pub const SCHEMA: u32 = 1;

fn schema() -> u32 {
    SCHEMA
}

/// One axle: where it is and how big its wheels are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxleDefinition {
    /// Position along the vehicle (m, positive forward).
    pub longitudinal_m: f32,
    /// Wheel diameter (m), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wheel_diameter_m: Option<f32>,
}

/// A vehicle as the engine sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleDefinition {
    /// File schema.
    #[serde(default = "schema")]
    pub schema: u32,
    /// Manufacturer (may be empty).
    #[serde(default)]
    pub manufacturer: String,
    /// Model designation.
    #[serde(default)]
    pub model: String,
    /// Default livery name (may be empty).
    #[serde(default)]
    pub livery: String,
    /// Empty mass (tonnes), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass_tonnes: Option<f32>,
    /// Axles, front to back as the source lists them.
    #[serde(default)]
    pub axles: Vec<AxleDefinition>,
}

impl Default for VehicleDefinition {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            manufacturer: String::new(),
            model: String::new(),
            livery: String::new(),
            mass_tonnes: None,
            axles: Vec::new(),
        }
    }
}

/// Why a native file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The text is not TOML of the vehicle shape.
    #[error("not a vehicle file: {0}")]
    Syntax(String),
}

impl VehicleDefinition {
    /// The TOML text of the definition.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a definition from TOML.
    ///
    /// # Errors
    /// The text is not TOML of the vehicle shape.
    pub fn from_toml(text: &str) -> Result<Self, ReadError> {
        toml::from_str(text).map_err(|e| ReadError::Syntax(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_survives_a_toml_round_trip() {
        let v = VehicleDefinition {
            manufacturer: "MAN".into(),
            model: "SD202".into(),
            livery: "Default".into(),
            mass_tonnes: Some(11.5),
            axles: vec![
                AxleDefinition {
                    longitudinal_m: 2.3,
                    wheel_diameter_m: Some(1.0),
                },
                AxleDefinition {
                    longitudinal_m: -3.1,
                    wheel_diameter_m: None,
                },
            ],
            ..Default::default()
        };
        let back = VehicleDefinition::from_toml(&v.to_toml()).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn missing_fields_take_defaults_and_garbage_is_an_error() {
        let v = VehicleDefinition::from_toml("model = \"X\"").unwrap();
        assert_eq!((v.schema, v.model.as_str(), v.axles.len()), (SCHEMA, "X", 0));
        assert!(VehicleDefinition::from_toml("= =").is_err());
    }
}
