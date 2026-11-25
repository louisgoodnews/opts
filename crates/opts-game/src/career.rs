//! Career and progression, the first slice (ISS-047, M-0167).
//!
//! The career keeps company money, the vehicles the player owns, a count of finished shifts and
//! experience. A shift is a stretch of the drive measured in simulated seconds (the session
//! counters). When one ends, its distance and length earn experience, the level is recomputed and
//! the shift count goes up. The state is saved by the `career` section (`save::sections::career`).
//!
//! Money is not earned yet: the cash desk model (M-0156) is the one money model and it is not built.
//! Until it is, `SHIFT_PAY_CENTS` is zero. No purchase flow exists, so `owned_vehicles` stays empty.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::app_state::AppState;
use crate::in_game::types::SimulationLoadState;
use crate::pause::SessionStats;

/// Simulated seconds in one shift.
// TODO(owner): ISS-047 gives no shift length. 30 minutes is a placeholder.
pub const SHIFT_LENGTH_S: f32 = 1800.0;
/// Experience for finishing a shift, whatever its distance.
// TODO(owner): experience values are not given by ISS-047.
pub const XP_PER_SHIFT: u32 = 50;
/// Experience per whole kilometre driven in a shift.
// TODO(owner): experience values are not given by ISS-047.
pub const XP_PER_KM: u32 = 10;
/// Pay in cents for finishing a shift. Zero until the M-0156 money model exists.
// TODO(owner): pay is not given by ISS-047; set it with the cash desk model (M-0156).
pub const SHIFT_PAY_CENTS: i64 = 0;
/// Experience needed for level 1, 2, 3 and so on (one entry per level above 0).
// TODO(owner): thresholds are not given by ISS-047.
pub const LEVEL_XP: [u32; 4] = [100, 250, 500, 1000];

/// A point in the session counters: where a shift starts or the drive is now.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShiftMark {
    /// Simulated seconds of the drive.
    pub duration_s: f32,
    /// Metres covered.
    pub distance_m: f32,
}

/// The career state. It is the resource and the data of the `career` save section.
#[derive(Resource, Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Career {
    /// Company money in cents.
    #[serde(default)]
    pub money_cents: i64,
    /// The vehicles the player owns, by `.bus` file name.
    #[serde(default)]
    pub owned_vehicles: Vec<String>,
    /// Shifts finished so far.
    #[serde(default)]
    pub shifts_done: u32,
    /// Experience so far.
    #[serde(default)]
    pub xp: u32,
    /// Where the running shift started. `None` before the first tick of a session.
    #[serde(default)]
    pub shift: Option<ShiftMark>,
}

/// A shift that ended on this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShiftDone {
    /// Kilometres driven in the shift.
    pub km: f32,
    /// Experience the shift earned.
    pub xp_gained: u32,
    /// The level before the shift counted.
    pub level_before: u32,
    /// The level after it.
    pub level_after: u32,
}

/// The level that `xp` reaches: the number of thresholds in `LEVEL_XP` that it meets.
#[must_use]
pub fn level_for(xp: u32) -> u32 {
    LEVEL_XP.iter().take_while(|&&need| xp >= need).count() as u32
}

impl Career {
    /// The current level.
    #[must_use]
    pub fn level(&self) -> u32 {
        level_for(self.xp)
    }

    /// Moves the running shift on to the session counters `now`. Returns the shift that ended on
    /// this call, if one did. The next shift starts at `now`.
    pub fn advance(&mut self, now: ShiftMark) -> Option<ShiftDone> {
        let start = match self.shift {
            // The counters went back (a new drive, or a restore before the counters): re-anchor.
            Some(start) if now.duration_s >= start.duration_s => start,
            _ => {
                self.shift = Some(now);
                return None;
            }
        };
        if now.duration_s - start.duration_s < SHIFT_LENGTH_S {
            return None;
        }
        let km = (now.distance_m - start.distance_m).max(0.0) / 1000.0;
        let level_before = self.level();
        let xp_gained = XP_PER_SHIFT.saturating_add((km as u32).saturating_mul(XP_PER_KM));
        self.xp = self.xp.saturating_add(xp_gained);
        self.shifts_done += 1;
        self.money_cents += SHIFT_PAY_CENTS;
        self.shift = Some(now);
        Some(ShiftDone {
            km,
            xp_gained,
            level_before,
            level_after: self.level(),
        })
    }
}

/// Advances the shift from the session counters. Runs while a session is loaded and running.
fn advance_career(
    stats: Res<SessionStats>,
    load: Res<SimulationLoadState>,
    mut career: ResMut<Career>,
) {
    if !load.is_complete {
        return;
    }
    let now = ShiftMark {
        duration_s: stats.duration_s,
        distance_m: stats.distance_m,
    };
    if let Some(done) = career.advance(now) {
        debug!(
            km = done.km,
            xp_gained = done.xp_gained,
            xp = career.xp,
            level = done.level_after,
            shifts_done = career.shifts_done,
            "shift finished: progression advanced"
        );
        if done.level_after > done.level_before {
            info!(
                level = done.level_after,
                xp = career.xp,
                shifts_done = career.shifts_done,
                "career milestone reached: new level"
            );
        }
    }
}

/// A session starts from a fresh career; a save restores its own afterwards.
fn reset_career(mut career: ResMut<Career>) {
    *career = Career::default();
}

/// Registers the career state and its tick. The save section is registered in `save::sections`.
#[derive(Default)]
pub struct CareerPlugin;

impl Plugin for CareerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Career>()
            .add_systems(
                Update,
                advance_career.run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), reset_career);
    }
}
