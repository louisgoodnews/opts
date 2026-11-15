//! The feed of an engine: the per-frame write and the name lists of a plan.

use super::super::*;
use super::{FeedPlan, Slot};

impl FeedPlan {
    /// Names the host feeds (not owned by the scripts).
    #[must_use]
    pub fn fed_names(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| !r.script_owned)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Names of the table the scripts store to themselves (the host leaves them alone).
    #[must_use]
    pub fn script_owned(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.script_owned)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Fed names that carry the accepted zero because no system produces them yet.
    #[must_use]
    pub fn zero_fed(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| !r.script_owned && !r.live)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Whether `name` (any case) is a table name, and whether this plan feeds it.
    #[must_use]
    pub fn status(&self, name: &str) -> Option<bool> {
        let lower = name.to_ascii_lowercase();
        self.rows
            .iter()
            .find(|r| r.name == lower)
            .map(|r| !r.script_owned)
    }
}

/// Writes every input of the plan into the engine. Names the scripts own are skipped.
pub fn feed(engine: &mut ScriptEngine, plan: &FeedPlan, inputs: &HostInputs) {
    for r in &plan.rows {
        if r.script_owned {
            continue;
        }
        let value = match r.slot {
            Slot::Scalar(i) => inputs.value(&INPUT_TABLE[i]),
            Slot::Wheel(family, axle, right) => inputs.wheel(family, axle, right),
            Slot::Door(0, d) => f32::from(u8::from(inputs.pax_entry_req[d])),
            Slot::Door(_, d) => f32::from(u8::from(inputs.pax_exit_req[d])),
        };
        match r.kind {
            Kind::Float => engine.set_id(r.id, value),
            Kind::System => engine.set_system_id(r.id, value),
        }
    }
}
