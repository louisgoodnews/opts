//! The pipeline of [`find_start`]: from a run's phase to where its train appears.

use super::super::*;
use opts_core::formats::timetable::ActiveSchedule;

/// Where the train of a run appears, counting the steps reached in `seen`; `Err(true)` when the
/// run is to be tried again later (no route), `Err(false)` when it is not to be placed now.
pub(in super::super) fn find_start(
    pass: &Pass,
    schedule: &ActiveSchedule,
    seen: &mut [u32; 6],
) -> Result<Found, bool> {
    let phase = start_phase(schedule, pass.now).ok_or(false)?;
    let lay = place_run(pass, schedule, phase).ok_or(false)?;
    seen[2] += 1;
    let (plan, start, skip_first) = route_run(pass, schedule, &lay).ok_or(true)?;
    seen[3] += 1;
    let (i, s) = appear_at(pass, &plan, start, (lay.offset, skip_first)).ok_or(false)?;
    Ok(Found {
        lay,
        plan,
        skip_first,
        i,
        s,
    })
}
