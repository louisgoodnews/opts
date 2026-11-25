//! The frame-time budget the governor judges against, and the display it comes from.

use super::*;

/// The frame-time budget (ms) the governor judges against.
///
/// * vsync on: the refresh interval of the window's monitor ([`FALLBACK_VSYNC_MS`] when
///   unknown), but never shorter than `1000 / target_fps` (the setting can ask for a lower
///   target, e.g. 30 fps on a 144 Hz display);
/// * vsync off: `1000 / target_fps`.
///
/// `target_fps` is clamped to [`MIN_TARGET_FPS`]`..=`[`MAX_TARGET_FPS`].
#[must_use]
pub fn target_ms(vsync: bool, refresh_millihertz: Option<u32>, target_fps: u32) -> f32 {
    let wanted = 1000.0 / target_fps.clamp(MIN_TARGET_FPS, MAX_TARGET_FPS) as f32;
    if vsync {
        let interval = refresh_millihertz
            .filter(|m| *m > 0)
            .map_or(FALLBACK_VSYNC_MS, |m| 1.0e6 / m as f32);
        interval.max(wanted)
    } else {
        wanted
    }
}

/// Whether a present mode waits for the vertical blank.
#[must_use]
pub fn is_vsync(mode: bevy::window::PresentMode) -> bool {
    use bevy::window::PresentMode as P;
    matches!(mode, P::Fifo | P::FifoRelaxed | P::AutoVsync)
}

/// The target frame rate the user asked for (`graphics.light_governor_target_fps`, written by
/// the game). Used as the deadline when vsync is off and as a lower bound of the target when
/// vsync is on.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernorTargetFps(pub u32);

impl Default for GovernorTargetFps {
    fn default() -> Self {
        Self(DEFAULT_TARGET_FPS)
    }
}

/// What the governor knows about the display this frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GovernorBudget {
    /// The deadline (ms), see [`target_ms`].
    pub target_ms: f32,
    /// Whether the window presents with vsync.
    pub vsync: bool,
    /// The refresh rate of the window's monitor, when known.
    pub refresh_millihertz: Option<u32>,
    /// False while the window is unfocused or minimised: no frames count then.
    pub window_active: bool,
}

impl Default for GovernorBudget {
    fn default() -> Self {
        Self {
            target_ms: FALLBACK_VSYNC_MS,
            vsync: false,
            refresh_millihertz: None,
            window_active: true,
        }
    }
}

/// Reads the primary window (present mode, focus, size) and its monitor (refresh rate) and
/// computes the [`GovernorBudget`]. Without a window (headless) the budget is the target fps.
pub fn update_governor_budget(
    windows: Query<(&Window, Option<&bevy::window::OnMonitor>), With<bevy::window::PrimaryWindow>>,
    monitors: Query<&bevy::window::Monitor>,
    primary_monitor: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
    fps: Res<GovernorTargetFps>,
    mut budget: ResMut<GovernorBudget>,
) {
    let want = match windows.single() {
        Ok((window, on)) => {
            let vsync = is_vsync(window.present_mode);
            let refresh = on
                .and_then(|m| monitors.get(m.0).ok())
                .or_else(|| primary_monitor.iter().next())
                .and_then(|m| m.refresh_rate_millihertz);
            GovernorBudget {
                target_ms: target_ms(vsync, refresh, fps.0),
                vsync,
                refresh_millihertz: refresh,
                window_active: window.focused
                    && window.resolution.physical_width() > 0
                    && window.resolution.physical_height() > 0,
            }
        }
        Err(_) => GovernorBudget {
            target_ms: target_ms(false, None, fps.0),
            ..GovernorBudget::default()
        },
    };
    if *budget != want {
        *budget = want;
    }
}
