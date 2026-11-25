//! The `career` save section (ISS-047, M-0167): money, owned vehicles, finished shifts, experience
//! and the running shift's mark. A file without it starts a fresh career. The section is applied
//! after `counters`, so the shift mark and the restored counters come from the same drive.

use bevy::prelude::*;
use serde_json::Value;
use tracing::debug;

use super::super::registry::{SaveSection, Stage};
use super::{parse, to_value};
use crate::career::Career;

/// The version this build writes and reads.
pub const CAREER_VERSION: u32 = 1;

/// The section registration; `base` supplies the defaults the section does not set.
pub(super) fn section(base: SaveSection) -> SaveSection {
    SaveSection {
        id: "career",
        version: CAREER_VERSION,
        stage: Stage::Settled,
        order: 25,
        capture: career_capture,
        apply: Some(career_apply),
        about: "money, owned vehicles, finished shifts, experience and the running shift",
        ..base
    }
}

fn career_capture(world: &mut World) -> Result<Option<Value>, String> {
    let Some(career) = world.get_resource::<Career>() else {
        return Ok(None);
    };
    to_value(career)
}

fn career_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let career: Career = parse(data, "career")?;
    debug!(
        xp = career.xp,
        shifts_done = career.shifts_done,
        "career restored from save"
    );
    world.insert_resource(career);
    Ok(())
}
