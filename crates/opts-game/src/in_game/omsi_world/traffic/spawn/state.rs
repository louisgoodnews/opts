//! The spawn pass's log line and the bookkeeping of unusable slots.

use super::*;

/// A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
pub(super) fn log_state(
    since_log: &mut f32,
    plan: &DensityPlan,
    state: &TrafficState,
    pool: &TrafficPool,
    rejected: &[u32; 5],
) {
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target = plan.target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            hour = plan.hour,
            day_bit = plan.day_bit,
            time_factors = ?plan.factors,
            wanted = ?plan.wanted,
            alive = ?plan.alive,
            capped = plan.capped,
            ready = count(|s| matches!(s, SlotState::Ready(_))),
            pending = count(|s| matches!(s, SlotState::Pending)),
            failed = count(|s| matches!(s, SlotState::Failed)),
            rejected = ?*rejected,
            "traffic state"
        );
    }
}

/// A vehicle whose meshes cannot be loaded now: a slot that can never load is failed, any other
/// gets a few more tries.
pub(super) fn mark_unusable(pool: &mut TrafficPool, slot: usize, unusable: bool) {
    if unusable {
        pool.slots[slot].state = SlotState::Failed;
    } else {
        pool.slots[slot].fails += 1;
        if pool.slots[slot].fails >= MAX_LOAD_TRIES {
            pool.slots[slot].state = SlotState::Failed;
        }
    }
}
