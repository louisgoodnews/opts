//! What the session summary may show: only measures that exist.

use bevy::prelude::*;
use opts_logging::LogQuery;

use crate::app_state::AppState;
use crate::in_game::types::SimulationLoadState;

/// Measures of the running drive.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SessionStats {
    /// Simulated seconds driven (virtual time, so a pause does not count).
    pub duration_s: f32,
    /// Distance covered in metres (the integral of the player's speed).
    pub distance_m: f32,
    /// Highest speed in m/s.
    pub max_speed_m_s: f32,
    /// Error-level log entries at the start of the drive.
    pub errors_at_start: usize,
    /// Error-level log entries now.
    pub errors_now: usize,
}

impl SessionStats {
    /// Problems logged during the drive (error-level log entries).
    pub fn problems(&self) -> usize {
        self.errors_now.saturating_sub(self.errors_at_start)
    }

    /// Maximum speed in km/h.
    pub fn max_speed_kmh(&self) -> f32 {
        self.max_speed_m_s * 3.6
    }

    /// Whether the vehicle moved at all (an unused drive is left without a summary).
    pub fn moved(&self) -> bool {
        self.distance_m >= 5.0 || self.max_speed_m_s >= 1.0
    }

    /// Adds one step of `dt` seconds at `speed` m/s.
    pub fn step(&mut self, dt: f32, speed: f32) {
        self.duration_s += dt;
        self.distance_m += speed.abs() * dt;
        self.max_speed_m_s = self.max_speed_m_s.max(speed.abs());
    }
}

/// `m:ss` or `h:mm:ss`.
pub fn format_duration(seconds: f32) -> String {
    let total = seconds.max(0.0) as u64;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn error_count() -> usize {
    let query = LogQuery {
        min_level: Some(tracing::Level::ERROR),
        ..Default::default()
    };
    opts_logging::recent_logs(100_000, &query).len()
}

/// A new drive starts with empty measures.
pub fn reset_stats(mut stats: ResMut<SessionStats>) {
    *stats = SessionStats {
        errors_at_start: error_count(),
        errors_now: error_count(),
        ..Default::default()
    };
}

/// Integrates the player's speed while the drive runs (not while loading or paused).
pub fn track_stats(
    time: Res<Time<Virtual>>,
    load: Res<SimulationLoadState>,
    player: Query<&opts_bevy::VehicleController, With<opts_bevy::PlayerVehicle>>,
    mut stats: ResMut<SessionStats>,
    mut since_log: Local<f32>,
) {
    if !load.is_complete || time.is_paused() {
        return;
    }
    let speed = player
        .single()
        .map_or(0.0, |c| c.sim.chassis.forward_speed_m_s());
    stats.step(time.delta_secs(), speed);
    *since_log += time.delta_secs();
    if *since_log > 2.0 {
        *since_log = 0.0;
        stats.errors_now = error_count();
    }
}

/// Registers the measures.
pub fn build(app: &mut App) {
    app.init_resource::<SessionStats>()
        .add_systems(OnEnter(AppState::InGame), reset_stats)
        .add_systems(Update, track_stats.run_if(in_state(AppState::InGame)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_accumulate_duration_distance_and_top_speed() {
        let mut s = SessionStats::default();
        s.step(1.0, 10.0);
        s.step(2.0, -5.0);
        assert_eq!(s.duration_s, 3.0);
        assert_eq!(s.distance_m, 20.0);
        assert_eq!(s.max_speed_m_s, 10.0);
        assert!((s.max_speed_kmh() - 36.0).abs() < 1e-3);
        assert!(s.moved());
        assert!(!SessionStats::default().moved());
    }

    #[test]
    fn problems_are_the_errors_since_the_start() {
        let s = SessionStats {
            errors_at_start: 3,
            errors_now: 5,
            ..Default::default()
        };
        assert_eq!(s.problems(), 2);
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(format_duration(65.0), "1:05");
        assert_eq!(format_duration(3725.0), "1:02:05");
        assert_eq!(format_duration(-3.0), "0:00");
    }
}
