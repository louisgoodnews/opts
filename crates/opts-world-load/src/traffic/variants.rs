//! Traffic vehicle variants: the reading and parsing of the AI lists, with the lane budget
//! ([`lanes`]), the slot choice ([`slot`]) and the unit geometry ([`unit`]) in children.

use super::*;

mod lanes;
mod slot;
#[cfg(test)]
mod tests;
mod unit;

pub(super) use lanes::baseline_targets;
pub(in super::super::super) use lanes::{group_lane_metres, time_factor};
pub(super) use slot::pick_slot;
pub(super) use unit::{unit_lod, unit_size};

/// Reads the map's AI lists once and parses one vehicle type every few frames.
pub fn load_traffic_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<TrafficSettings>,
    mut pool: ResMut<TrafficPool>,
) {
    let Some(job) = job else { return };
    if settings.baseline == 0 {
        return;
    }
    if !pool.started {
        pool.started = true;
        pool.rng = Some(Rng::new(0xA11_CA45));
        let PoolSetup {
            groups,
            slots,
            defaults,
            depots,
        } = read_groups(&job);
        pool.groups = groups;
        pool.slots = slots;
        pool.defaults = defaults;
        pool.depots = depots;
        pool.road_curve = DensityProfile {
            days: 0,
            points: job.manifest.traffic_density_road.clone(),
        };
    }
    pool.frames += 1;
    if !pool.frames.is_multiple_of(10) {
        return;
    }
    let Some(i) = pool
        .slots
        .iter()
        .position(|s| matches!(s.state, SlotState::Pending))
    else {
        return;
    };
    let file = pool.slots[i].file.clone();
    match load_variant(&job, &file) {
        Ok(v) => pool.slots[i].state = SlotState::Ready(Box::new(v)),
        Err(e) => {
            tracing::debug!(vehicle = %file, "AI vehicle skipped: {e}");
            pool.slots[i].state = SlotState::Failed;
        }
    }
}
