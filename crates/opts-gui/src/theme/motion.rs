//! Motion tokens, easing curves and the transition system that consumes them.
//!
//! Durations (Phase 9, `design-language.md` section 8): hover and focus 100 ms
//! ([`DurationToken::Short`]), panels and dialogs 250 ms ([`DurationToken::Medium`]), screen
//! changes 400 ms ([`DurationToken::Long`]). **Reduced motion** (the `reduced_motion` setting or
//! the operating system's hint, see [`MotionPreference`]) turns every transition into an instant
//! change.
//!
//! Usage: insert a [`BackgroundTransition`] on an entity with a `BackgroundColor`; the
//! [`drive_background_transitions`] system eases the colour and removes the component when done.
//! The theme switch itself is deliberately **not** animated (it is one frame, as specified in
//! `theme-auto-switch.md`).

use std::time::Duration;

use bevy::prelude::*;

use super::theme::MaterialTheme;

/// Standard animation and transition duration tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurationToken {
    /// 100 ms: hover, focus, press feedback.
    Short,
    /// 250 ms: panels, dialogs, menus.
    Medium,
    /// 400 ms: screen changes.
    Long,
}

impl DurationToken {
    /// Duration in milliseconds.
    pub const fn millis(&self) -> u64 {
        match self {
            Self::Short => 100,
            Self::Medium => 250,
            Self::Long => 400,
        }
    }

    /// Converts to standard Rust [`Duration`].
    pub const fn as_duration(&self) -> Duration {
        Duration::from_millis(self.millis())
    }
}

/// What is moving, which decides the duration (so call sites say *what*, the theme says *how
/// long*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionRole {
    /// A control reacts to hover, focus or press: [`DurationToken::Short`].
    HoverFocus,
    /// A panel, menu or dialog appears or disappears: [`DurationToken::Medium`].
    Panel,
    /// A whole screen changes: [`DurationToken::Long`].
    Screen,
}

impl MotionRole {
    /// The duration token of the role.
    pub const fn token(self) -> DurationToken {
        match self {
            Self::HoverFocus => DurationToken::Short,
            Self::Panel => DurationToken::Medium,
            Self::Screen => DurationToken::Long,
        }
    }

    /// The easing of the role: standard for small reactions, emphasised for screens.
    pub const fn easing(self) -> EasingCurve {
        match self {
            Self::HoverFocus | Self::Panel => EasingCurve::Standard,
            Self::Screen => EasingCurve::Emphasized,
        }
    }
}

/// Standard easing curves (cubic Bezier as in Material 3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingCurve {
    /// `cubic-bezier(0.2, 0, 0, 1)`.
    Standard,
    /// `cubic-bezier(0.05, 0.7, 0.1, 1)`.
    Emphasized,
    /// Entering: `cubic-bezier(0, 0, 0, 1)`.
    Decelerated,
    /// Leaving: `cubic-bezier(0.3, 0, 1, 1)`.
    Accelerated,
}

impl EasingCurve {
    const fn control_points(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Standard => (0.2, 0.0, 0.0, 1.0),
            Self::Emphasized => (0.05, 0.7, 0.1, 1.0),
            Self::Decelerated => (0.0, 0.0, 0.0, 1.0),
            Self::Accelerated => (0.3, 0.0, 1.0, 1.0),
        }
    }

    /// Maps linear progress `t` in `[0, 1]` to eased progress in `[0, 1]`.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let (x1, y1, x2, y2) = self.control_points();
        let bez = |a: f32, b: f32, s: f32| {
            let inv = 1.0 - s;
            3.0 * inv * inv * s * a + 3.0 * inv * s * s * b + s * s * s
        };
        // Solve x(s) = t by bisection, then evaluate y(s).
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..24 {
            let mid = 0.5 * (lo + hi);
            if bez(x1, x2, mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        bez(y1, y2, 0.5 * (lo + hi))
    }
}

/// Motion configuration tokens.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionTokens {
    /// When set, every duration is zero.
    pub reduced_motion: bool,
}

impl MotionTokens {
    /// Retrieves the duration, respecting reduced motion mode.
    pub fn duration(&self, token: DurationToken) -> Duration {
        if self.reduced_motion {
            Duration::ZERO
        } else {
            token.as_duration()
        }
    }

    /// The duration of a [`MotionRole`], respecting reduced motion mode.
    pub fn duration_for(&self, role: MotionRole) -> Duration {
        self.duration(role.token())
    }
}

/// Where the "reduce motion" wish comes from: the user's setting and the operating system's hint.
/// Either one is enough.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MotionPreference {
    /// The `reduced_motion` setting.
    pub user_setting: bool,
    /// The OS accessibility hint (`false` when unknown; set by the platform probe in the game).
    pub os_hint: bool,
}

impl MotionPreference {
    /// `true` when motion should be reduced.
    pub const fn reduced(self) -> bool {
        self.user_setting || self.os_hint
    }
}

/// Copies [`MotionPreference`] into [`MaterialTheme::motion`] when either changes.
pub fn sync_motion_preference(
    preference: Option<Res<MotionPreference>>,
    mut theme: ResMut<MaterialTheme>,
) {
    let Some(preference) = preference else { return };
    if preference.is_changed() && theme.motion.reduced_motion != preference.reduced() {
        theme.motion.reduced_motion = preference.reduced();
    }
}

/// An eased change of an entity's [`BackgroundColor`] from `from` to `to`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BackgroundTransition {
    /// Colour at the start.
    pub from: Color,
    /// Colour at the end.
    pub to: Color,
    /// Duration (the role's token, before the reduced-motion decision).
    pub role: MotionRole,
    /// Time elapsed.
    pub elapsed: Duration,
}

impl BackgroundTransition {
    /// Starts a transition for a motion role.
    pub const fn new(from: Color, to: Color, role: MotionRole) -> Self {
        Self {
            from,
            to,
            role,
            elapsed: Duration::ZERO,
        }
    }
}

/// Advances every [`BackgroundTransition`] with real time (so interface motion keeps running while
/// the simulation is paused) and removes it when finished or when reduced motion is on.
pub fn drive_background_transitions(
    mut commands: Commands,
    time: Res<Time<Real>>,
    theme: Res<MaterialTheme>,
    mut query: Query<(Entity, &mut BackgroundTransition, &mut BackgroundColor)>,
) {
    for (entity, mut transition, mut background) in &mut query {
        let total = theme.motion.duration_for(transition.role);
        transition.elapsed += time.delta();
        if total.is_zero() || transition.elapsed >= total {
            background.0 = transition.to;
            commands.entity(entity).remove::<BackgroundTransition>();
            continue;
        }
        let t = transition.elapsed.as_secs_f32() / total.as_secs_f32();
        let eased = transition.role.easing().apply(t);
        background.0 = transition.from.mix(&transition.to, eased);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_match_the_tokens() {
        assert_eq!(MotionRole::HoverFocus.token().millis(), 100);
        assert_eq!(MotionRole::Panel.token().millis(), 250);
        assert_eq!(MotionRole::Screen.token().millis(), 400);
    }

    #[test]
    fn reduced_motion_zeroes_every_duration() {
        let m = MotionTokens {
            reduced_motion: true,
        };
        for r in [
            MotionRole::HoverFocus,
            MotionRole::Panel,
            MotionRole::Screen,
        ] {
            assert_eq!(m.duration_for(r), Duration::ZERO);
        }
        assert_eq!(
            MotionTokens::default().duration_for(MotionRole::Panel),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn either_source_reduces_motion() {
        assert!(!MotionPreference::default().reduced());
        assert!(MotionPreference {
            user_setting: true,
            os_hint: false
        }
        .reduced());
        assert!(MotionPreference {
            user_setting: false,
            os_hint: true
        }
        .reduced());
    }

    #[test]
    fn easing_is_monotonic_and_hits_the_ends() {
        for e in [
            EasingCurve::Standard,
            EasingCurve::Emphasized,
            EasingCurve::Decelerated,
            EasingCurve::Accelerated,
        ] {
            assert!(e.apply(0.0).abs() < 1e-3);
            assert!((e.apply(1.0) - 1.0).abs() < 1e-3);
            let mut prev = 0.0;
            for i in 1..=20 {
                let v = e.apply(i as f32 / 20.0);
                assert!(v + 1e-4 >= prev, "{e:?} not monotonic");
                prev = v;
            }
        }
    }

    fn app(reduced: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).insert_resource(
            bevy::time::TimeUpdateStrategy::ManualDuration(Duration::ZERO),
        );
        let mut theme = MaterialTheme::default();
        theme.motion.reduced_motion = reduced;
        app.insert_resource(theme)
            .add_systems(Update, drive_background_transitions);
        app
    }

    #[test]
    fn a_transition_ends_on_the_target_colour_and_removes_itself() {
        let mut app = app(false);
        let e = app
            .world_mut()
            .spawn((
                BackgroundColor(Color::BLACK),
                BackgroundTransition::new(Color::BLACK, Color::WHITE, MotionRole::HoverFocus),
            ))
            .id();
        app.update();
        // advance real time by more than 100 ms
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(150),
        ));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(e).map(|b| b.0),
            Some(Color::WHITE)
        );
        assert!(app.world().get::<BackgroundTransition>(e).is_none());
    }

    #[test]
    fn reduced_motion_jumps_straight_to_the_target() {
        let mut app = app(true);
        let e = app
            .world_mut()
            .spawn((
                BackgroundColor(Color::BLACK),
                BackgroundTransition::new(Color::BLACK, Color::WHITE, MotionRole::Panel),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(e).map(|b| b.0),
            Some(Color::WHITE)
        );
        assert!(app.world().get::<BackgroundTransition>(e).is_none());
    }
}
