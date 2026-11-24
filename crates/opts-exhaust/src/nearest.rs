//! Which vehicles put out exhaust (M-0117): the player's vehicle and the [`NEAREST_N`] nearest other
//! vehicles with an outlet, and the distance fade of their puff rate. Pure choice and numbers; the
//! system reads the distances.

/// The other vehicles (besides the player's) that put out puffs at a time: the nearest to the
/// camera. Four, so that five full-load plumes (the player and four) fit the top of the setting's
/// cap range: one full-load vehicle keeps about 6 puffs/s x 1.5 s = 9 puffs alive, and 5 x 9 = 45
/// is within the 48 of `graphics.exhaust_particles`. Not measured against the frame-time target.
pub const NEAREST_N: usize = 4;
/// Distance (m) to the camera up to which a vehicle puffs at its full rate.
pub const FADE_START_M: f32 = 25.0;
/// Distance (m) to the camera at which the puff rate has faded to zero. Beyond it a puff of 0.35
/// to 0.7 m is a few pixels wide at the default view (estimated, not measured).
pub const FADE_END_M: f32 = 100.0;
/// Engine load of an AI vehicle, which has no throttle: a steady cruise (0 idle, 1 full throttle).
pub const AI_LOAD: f32 = 0.3;

/// The fraction of the full puff rate at `distance_m` from the camera: 1 up to [`FADE_START_M`],
/// linear down to 0 at [`FADE_END_M`], and 0 beyond.
#[must_use]
pub fn fade_of(distance_m: f32) -> f32 {
    ((FADE_END_M - distance_m) / (FADE_END_M - FADE_START_M)).clamp(0.0, 1.0)
}

/// The vehicles that put out puffs this frame, player first: the player's (`player`, always
/// included when there is one) and the `n` nearest of `others`, each with its distance to the
/// camera. `others` does not hold the player.
#[must_use]
pub fn nearest<K: Copy>(
    player: Option<(K, f32)>,
    mut others: Vec<(K, f32)>,
    n: usize,
) -> Vec<(K, f32)> {
    others.sort_by(|a, b| a.1.total_cmp(&b.1));
    player
        .into_iter()
        .chain(others.into_iter().take(n))
        .collect()
}

#[cfg(test)]
mod tests;
