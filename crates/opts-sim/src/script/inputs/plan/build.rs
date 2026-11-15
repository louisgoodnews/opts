//! Resolving the feed of an engine once: the variable ids, the names the scripts own, the slots.

use super::super::*;
use super::{FeedPlan, Planned, Slot};

impl FeedPlan {
    /// Resolves every table name against `engine` (names are interned if missing, so the
    /// scripts' later reads find the fed value).
    pub fn new(engine: &mut ScriptEngine) -> Self {
        let mut rows = Vec::new();
        let mut push = |engine: &mut ScriptEngine, name: String, slot, kind, shared, live| {
            let (id, script_owned) = match kind {
                // The pedals are fed every frame, also when a script writes them: a script that
                // writes `Throttle` for a corner case (the HH109 1992's idle assist for one bus
                // number) used to take the pedal away from the player for good.
                Kind::Float => (
                    engine.variable_id(&name),
                    shared && engine.writes(&name) && name != "throttle" && name != "brake",
                ),
                Kind::System => (engine.system_id(&name), false),
            };
            rows.push(Planned {
                name,
                slot,
                kind,
                id,
                script_owned,
                live,
            });
        };
        for (i, r) in INPUT_TABLE.iter().enumerate() {
            push(
                engine,
                r.name.to_owned(),
                Slot::Scalar(i),
                r.kind,
                r.shared,
                r.live,
            );
        }
        for (family, stem, _) in WHEEL_FAMILIES {
            let shared = *family == WheelFamily::Rotation;
            for axle in 0..AXLES {
                for (side, right) in [("l", false), ("r", true)] {
                    push(
                        engine,
                        format!("{stem}_{axle}_{side}"),
                        Slot::Wheel(*family, axle, right),
                        Kind::Float,
                        shared,
                        *family != WheelFamily::Suspension,
                    );
                }
            }
        }
        for (f, stem) in DOOR_FAMILIES.iter().enumerate() {
            for door in 0..DOORS {
                push(
                    engine,
                    format!("{stem}{door}_req"),
                    Slot::Door(f, door),
                    Kind::Float,
                    true,
                    false,
                );
            }
        }
        Self { rows }
    }
}
