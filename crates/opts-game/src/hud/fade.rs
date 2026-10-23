//! Fading of the status bar, the hints and the camera label.
//!
//! A pill with [`HudFade`] shows for `hold` seconds, fades for `fade` seconds and then hides.
//! The opacity reaches the texts and panels through [`InstrumentFade`], so every colour still
//! comes from the instrument tokens. With reduced motion the fade becomes a plain hold of the
//! same total time.

use bevy::prelude::*;
use opts_gui::theme::MaterialTheme;

use crate::in_game::instruments::InstrumentFade;

/// Seconds the camera label is visible in total (interaction-and-feel section 8).
pub const CAMERA_LABEL_SECONDS: f32 = 1.5;
/// Of which fading.
pub const CAMERA_LABEL_FADE: f32 = 0.3;
/// Seconds the status bar stays after the start and after each resume.
pub const STATUS_HOLD: f32 = 8.0;
/// Fade of the status bar.
pub const STATUS_FADE: f32 = 1.5;
/// Seconds the key hints stay after the start.
pub const HINTS_HOLD: f32 = 12.0;
/// Fade of the hints.
pub const HINTS_FADE: f32 = 2.0;

/// Fade state of one pill.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct HudFade {
    /// Seconds since it was shown.
    pub elapsed: f32,
    /// Seconds fully visible.
    pub hold: f32,
    /// Seconds of fading after the hold.
    pub fade: f32,
    /// Stays fully visible while true (the status bar while the game is paused).
    pub pinned: bool,
    last: f32,
}

impl HudFade {
    /// A pill that starts hidden (shown by [`show`](Self::show)).
    #[must_use]
    pub fn hidden(hold: f32, fade: f32) -> Self {
        Self {
            elapsed: hold + fade,
            hold,
            fade,
            pinned: false,
            last: -1.0,
        }
    }

    /// A pill that starts visible.
    #[must_use]
    pub fn shown(hold: f32, fade: f32) -> Self {
        Self {
            elapsed: 0.0,
            ..Self::hidden(hold, fade)
        }
    }

    /// Starts the hold again.
    pub fn show(&mut self) {
        self.elapsed = 0.0;
    }

    /// Opacity now, 0 to 1.
    #[must_use]
    pub fn alpha(&self, reduced_motion: bool) -> f32 {
        if self.pinned {
            return 1.0;
        }
        let (hold, fade) = if reduced_motion {
            (self.hold + self.fade, 0.0)
        } else {
            (self.hold, self.fade)
        };
        if self.elapsed < hold {
            1.0
        } else if fade > 0.0 && self.elapsed < hold + fade {
            1.0 - (self.elapsed - hold) / fade
        } else {
            0.0
        }
    }
}

/// Advances the fades and applies the opacity to the pill and everything in it.
pub fn drive_fades(
    time: Res<Time<Real>>,
    theme: Res<MaterialTheme>,
    mut pills: Query<(Entity, &mut HudFade, &mut Visibility)>,
    children: Query<&Children>,
    mut fades: Query<&mut InstrumentFade>,
) {
    let reduced = theme.motion.reduced_motion;
    for (entity, mut fade, mut vis) in &mut pills {
        if !fade.pinned {
            fade.elapsed += time.delta_secs();
        }
        let alpha = fade.alpha(reduced);
        if (alpha - fade.last).abs() < 0.004 {
            continue;
        }
        fade.last = alpha;
        let want = if alpha <= 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
        for e in std::iter::once(entity).chain(children.iter_descendants(entity)) {
            if let Ok(mut f) = fades.get_mut(e) {
                f.0 = alpha;
            }
        }
    }
}

/// Which pill a [`HudFade`] belongs to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeKind {
    /// The status bar.
    Status,
    /// The key hints.
    Hints,
    /// The camera label.
    CameraLabel,
}

/// Pins the status bar while the simulation is paused or a timetable is followed and restarts its hold on resume.
pub fn status_follows_pause(
    time: Res<Time<Virtual>>,
    values: Res<super::values::HudValues>,
    mut pills: Query<(&FadeKind, &mut HudFade)>,
    mut was_paused: Local<bool>,
) {
    // The status bar also stays while a timetable is followed: it carries the next stop.
    let paused = time.is_paused() || values.timetable.is_some();
    if paused == *was_paused {
        return;
    }
    *was_paused = paused;
    for (kind, mut fade) in &mut pills {
        if *kind == FadeKind::Status {
            fade.pinned = paused;
            if !paused {
                fade.show();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_label_is_visible_for_one_and_a_half_seconds() {
        let mut f = HudFade::hidden(CAMERA_LABEL_SECONDS - CAMERA_LABEL_FADE, CAMERA_LABEL_FADE);
        assert_eq!(f.alpha(false), 0.0);
        f.show();
        f.elapsed = 1.0;
        assert_eq!(f.alpha(false), 1.0);
        f.elapsed = 1.35;
        assert!(f.alpha(false) > 0.0 && f.alpha(false) < 1.0);
        f.elapsed = CAMERA_LABEL_SECONDS + 0.01;
        assert_eq!(f.alpha(false), 0.0);
    }

    #[test]
    fn reduced_motion_holds_for_the_same_total_without_fading() {
        let mut f = HudFade::shown(1.2, 0.3);
        f.elapsed = 1.4;
        assert_eq!(f.alpha(true), 1.0);
        f.elapsed = 1.6;
        assert_eq!(f.alpha(true), 0.0);
    }

    #[test]
    fn a_pinned_pill_does_not_fade() {
        let mut f = HudFade::shown(1.0, 1.0);
        f.elapsed = 99.0;
        f.pinned = true;
        assert_eq!(f.alpha(false), 1.0);
    }
}
