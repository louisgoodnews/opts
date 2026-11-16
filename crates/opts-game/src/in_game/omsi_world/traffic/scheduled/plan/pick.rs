//! The depot slot a scheduled run takes its vehicle from: one that is ready, chosen by the run.

use super::super::super::run_schedule::RunRef;
use super::super::super::TrafficPool;
use super::super::ScheduledTraffic;
use crate::in_game::omsi_world::traffic_variant::SlotState;

/// The vehicle slot of the tour's depot group: one that is ready, chosen by the run. Asks for one
/// to be parsed when none is ready, and gives the run up when none can be.
pub(super) fn pick_slot_for(
    (sched, pool): (&mut ScheduledTraffic, &mut TrafficPool),
    tt: &opts_core::formats::timetable::Timetables,
    (run, schedule): (&RunRef, &opts_core::formats::timetable::ActiveSchedule),
) -> Option<usize> {
    let tour = tt.tour(&schedule.line_file, run.tour)?;
    let Some(choices) = pool
        .depots
        .get(&tour.vehicle_group.to_ascii_lowercase())
        .cloned()
    else {
        sched.runs.entry(run.clone()).or_default().tries = 3;
        return None;
    };
    let ready: Vec<(usize, u32)> = choices
        .iter()
        .copied()
        .filter(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Ready(_)))
        .collect();
    if ready.is_empty() {
        // Ask for one to be parsed; the run is tried again next pass.
        if let Some((slot, _)) = choices
            .iter()
            .find(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Idle))
        {
            pool.slots[*slot].state = SlotState::Pending;
        } else if choices
            .iter()
            .all(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Failed))
        {
            sched.runs.entry(run.clone()).or_default().tries = 3;
        }
        return None;
    }
    let total: u32 = ready.iter().map(|(_, n)| n).sum();
    let mut pick = (run.run as u32 + run.tour as u32 * 7) % total.max(1);
    let slot = ready
        .iter()
        .find(|(_, n)| {
            if pick < *n {
                true
            } else {
                pick -= n;
                false
            }
        })
        .map_or(ready[0].0, |(s, _)| *s);
    Some(slot)
}
