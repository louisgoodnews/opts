//! The registry as the control protocol and the session save read and write it.

use serde_json::{json, Value};

use super::store::SessionVehicles;
use super::types::{Lifecycle, SessionVehicleRecord, SAVE_SECTION_VERSION};

impl SessionVehicles {
    /// The registry as the control protocol answers it.
    #[must_use]
    pub fn describe(&self) -> Value {
        json!({
            "active": self.active,
            "count": self.live_count(),
            "vehicles": self
                .records
                .iter()
                .map(|r| r.describe(self.active == Some(r.id)))
                .collect::<Vec<_>>(),
        })
    }

    /// The registry as a section of the session save: the vehicles that are in the world or on
    /// their way (the despawned ones are history, not state). The runtime fields are left out.
    #[must_use]
    pub fn to_section(&self) -> Value {
        let live: Vec<&SessionVehicleRecord> = self
            .records
            .iter()
            .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
            .collect();
        json!({
            "version": SAVE_SECTION_VERSION,
            "next_id": self.next_id,
            "active": self.active.filter(|a| live.iter().any(|r| r.id == *a)),
            "vehicles": live,
        })
    }

    /// Reads a section written by [`Self::to_section`]. The vehicles come back as spawn requests
    /// ([`Lifecycle::Spawning`]) at their saved spawn point; the spawner builds them again and the
    /// saved active id is restored once it exists.
    ///
    /// # Errors
    /// The section has another version or is not shaped like one.
    pub fn from_section(section: &Value) -> Result<Self, String> {
        let version = section.get("version").and_then(Value::as_u64);
        if version != Some(u64::from(SAVE_SECTION_VERSION)) {
            return Err(format!("unsupported session_vehicles version {version:?}"));
        }
        let records: Vec<SessionVehicleRecord> = serde_json::from_value(
            section
                .get("vehicles")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        )
        .map_err(|e| e.to_string())?;
        let next_id = section
            .get("next_id")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
            .max(records.iter().map(|r| r.id).max().unwrap_or(0));
        let active = section
            .get("active")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok());
        Ok(Self {
            next_id,
            active,
            records: records
                .into_iter()
                .map(|mut r| {
                    r.lifecycle = Lifecycle::Spawning;
                    r
                })
                .collect(),
        })
    }
}
