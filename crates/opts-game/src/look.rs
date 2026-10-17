//! The colour *look* of the 3D view: tonemapper, post-saturation, contrast and ambient scale.
//!
//! A [`Look`] is a user setting (settings screen, `--look`, control protocol `render.look`) that
//! maps to [`LookParams`]. The [`ActiveLook`] resource holds the look in effect; [`LookPlugin`]
//! applies its parameters live to the [`MainCamera`](opts_bevy::MainCamera) ([`apply_look_to_cameras`]) and the sky system
//! reads [`LookParams::ambient_scale`]. Provisional values, see M-0060 step 1 (physical calibration
//! follows in step 2): *Vivid* is the default, *Bevy default* reproduces the behaviour before the
//! looks existed.
//!
//! Spec: `docs/src/architecture/texture-quality.md` §3.2 and `docs/src/roadmap/phase-8/M-0060.md`.

use bevy::prelude::*;
use bevy::render::view::{ColorGrading, Tonemapping};
use serde::{Deserialize, Serialize};

/// A named colour look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Look {
    /// Richer saturation and contrast than reality-neutral (default).
    #[default]
    Vivid,
    /// Bevy's default tonemapper with the current ambient light; the base for the physical
    /// calibration of M-0060 step 2.
    Natural,
    /// Exactly the behaviour before looks existed.
    BevyDefault,
}

impl Look {
    /// All looks in menu order.
    pub const ALL: [Self; 3] = [Self::Vivid, Self::Natural, Self::BevyDefault];

    /// The stable key used in settings, the control protocol and `--look`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Vivid => "vivid",
            Self::Natural => "natural",
            Self::BevyDefault => "bevy",
        }
    }

    /// The label shown in the settings screen.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vivid => "Vivid (default)",
            Self::Natural => "Natural",
            Self::BevyDefault => "Bevy default",
        }
    }

    /// Parses a [`key`](Self::key) (also `bevy_default`, `bevy-default`; case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "vivid" => Some(Self::Vivid),
            "natural" => Some(Self::Natural),
            "bevy" | "bevy_default" | "bevy-default" | "bevydefault" => Some(Self::BevyDefault),
            _ => None,
        }
    }
}

/// What a [`Look`] does to the camera and the sky.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookParams {
    /// Tonemapper of the main camera.
    pub tonemapping: Tonemapping,
    /// Saturation applied after tonemapping (`1.0` = unchanged).
    pub post_saturation: f32,
    /// Contrast of shadows, midtones and highlights (`1.0` = unchanged).
    pub contrast: f32,
    /// Factor on the ambient light the sky system computes (`1.0` = unchanged).
    pub ambient_scale: f32,
}

impl Look {
    /// The parameters of this look (provisional values, M-0060 step 1).
    ///
    /// *Natural* and *Bevy default* are identical until the physical calibration of step 2 gives
    /// *Natural* its own sun, sky and exposure.
    #[must_use]
    pub fn params(self) -> LookParams {
        match self {
            // Measured on the reference view (texture-quality.md 3.2): +40 % mean saturation,
            // +20 % contrast against the Bevy default.
            Self::Vivid => LookParams {
                tonemapping: Tonemapping::SomewhatBoringDisplayTransform,
                post_saturation: 1.1,
                contrast: 1.0,
                ambient_scale: 0.5,
            },
            Self::Natural | Self::BevyDefault => LookParams {
                tonemapping: Tonemapping::default(),
                post_saturation: 1.0,
                contrast: 1.0,
                ambient_scale: 1.0,
            },
        }
    }
}

/// The look in effect. Starts as the setting (or the `--look` override of this run) and changes
/// live through the settings screen and the control protocol.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveLook(pub Look);

/// Applies the render options at start: the stored settings, overridden for this run by
/// `--texture-filter` / `--look` (the overrides are not persisted).
pub fn apply_startup_render_options(
    app: &mut App,
    settings: &crate::settings::GameSettings,
    args: &opts_cli::CliArgs,
) {
    let filtering = match args.texture_filter.as_deref() {
        Some(text) => opts_bevy::TextureFiltering::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --texture-filter (off, 4, 8, 16); using the setting"
            );
            settings.texture_filtering
        }),
        None => settings.texture_filtering,
    };
    opts_bevy::set_texture_filtering(filtering);
    let look = match args.look.as_deref() {
        Some(text) => Look::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --look (vivid, natural, bevy); using the setting"
            );
            settings.look
        }),
        None => settings.look,
    };
    tracing::info!(
        texture_filtering = filtering.key(),
        look = look.key(),
        "render options"
    );
    app.insert_resource(ActiveLook(look));
}

/// How much of the day's light is up: 0 at and below the horizon, 1 from 25 degrees of sun
/// elevation on (smooth in between).
#[must_use]
pub fn daylight(sun_elevation_deg: f32) -> f32 {
    let t = (sun_elevation_deg / 25.0).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The ambient scale of a look at the given sun elevation. A look darkens the ambient light to
/// deepen shadows in daylight; at dusk and night the base ambient stays (the ambient floor is what
/// keeps the scene readable), so the scale fades to 1 as the sun sets.
#[must_use]
pub fn effective_ambient_scale(look_scale: f32, sun_elevation_deg: f32) -> f32 {
    1.0 + (look_scale - 1.0) * daylight(sun_elevation_deg)
}

/// The colour grading of `params`; everything else (exposure, temperature, ...) stays at Bevy's
/// defaults so switching looks never leaves residue.
#[must_use]
pub fn grading_for(params: LookParams) -> ColorGrading {
    let mut g = ColorGrading::default();
    g.global.post_saturation = params.post_saturation;
    for section in [&mut g.shadows, &mut g.midtones, &mut g.highlights] {
        section.contrast = params.contrast;
    }
    g
}

/// CPU replica of Bevy's `SomewhatBoringDisplayTransform` (`bevy_core_pipeline` tonemapping.wesl).
fn somewhat_boring(c: [f32; 3]) -> [f32; 3] {
    let curve = |v: f32| 1.0 - (-v).exp();
    let lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let cb = -0.1146 * c[0] - 0.3854 * c[1] + 0.5 * c[2];
    let cr = 0.5 * c[0] - 0.4542 * c[1] - 0.0458 * c[2];
    let bt = curve(cb.hypot(cr) * 2.4);
    let desat = ((bt - 0.7) * 0.8).max(0.0).powi(2);
    let k = (curve(lum) / lum.max(1e-5)).max(0.0);
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let tm0 = c[i] * k;
        let tm1 = curve(c[i] + (lum - c[i]) * desat);
        *o = (tm0 + (tm1 - tm0) * bt * bt) * 0.97;
    }
    out
}

/// The scene-linear colour a camera with `Hdr` must clear to so that, after its tonemapper, the
/// sky region shows `display` (linear RGB `0..=1`), exactly what the 8-bit camera showed when the
/// clear colour bypassed the tonemapper (it was written straight to the target).
///
/// `SomewhatBoringDisplayTransform` is inverted numerically with its CPU replica, `None` is the
/// identity, every other tonemapper is approximated by an inverse Reinhard curve `x / (1 + x)` (an
/// approximation: the LUT based ones cannot be evaluated on the CPU). Colour grading (the
/// `Vivid` post saturation of 1.1) is not inverted.
#[must_use]
pub fn hdr_clear_color(tonemapping: Tonemapping, display: [f32; 3]) -> [f32; 3] {
    // The curves reach 1 only at infinity; stay below the white the target can show.
    let target = display.map(|v| v.clamp(0.0, 0.97));
    match tonemapping {
        Tonemapping::None => display,
        Tonemapping::SomewhatBoringDisplayTransform => {
            let target = display.map(|v| v.clamp(0.0, 0.9));
            let mut x = target.map(|v| v.max(1e-4));
            for _ in 0..48 {
                let y = somewhat_boring(x);
                for i in 0..3 {
                    x[i] = (x[i] * target[i] / y[i].max(1e-5)).clamp(1e-4, 64.0);
                }
            }
            x
        }
        _ => target.map(|v| v / (1.0 - v)),
    }
}

/// Applies the active look to the `MainCamera`: all of them when the look changed, new ones when
/// they appear.
pub fn apply_look_to_cameras(
    look: Res<ActiveLook>,
    cameras: Query<(Entity, Has<LookApplied>), With<opts_bevy::MainCamera>>,
    mut commands: Commands,
) {
    let params = look.0.params();
    let changed = look.is_changed();
    for (entity, applied) in &cameras {
        if changed || !applied {
            commands
                .entity(entity)
                .insert((params.tonemapping, grading_for(params), LookApplied));
        }
    }
}

/// Marks a camera that already carries the active look.
#[derive(Component)]
pub struct LookApplied;

/// Registers [`ActiveLook`] and the system that applies it.
pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLook>()
            .add_systems(Update, apply_look_to_cameras);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reinhard(x: f32) -> f32 {
        x / (1.0 + x)
    }

    #[test]
    fn vivid_is_the_default_and_keys_round_trip() {
        assert_eq!(Look::default(), Look::Vivid);
        for look in Look::ALL {
            assert_eq!(Look::parse(look.key()), Some(look));
        }
        assert_eq!(Look::parse(" Bevy_Default "), Some(Look::BevyDefault));
        assert_eq!(Look::parse("sepia"), None);
    }

    #[test]
    fn bevy_default_is_exactly_todays_behaviour() {
        let p = Look::BevyDefault.params();
        assert_eq!(p.tonemapping, Tonemapping::default());
        assert_eq!(grading_for(p), ColorGrading::default());
        assert_eq!(p.ambient_scale, 1.0);
        assert_eq!(Look::Natural.params(), p, "until the calibration of step 2");
    }

    #[test]
    fn vivid_uses_the_provisional_values() {
        let p = Look::Vivid.params();
        assert_eq!(p.tonemapping, Tonemapping::SomewhatBoringDisplayTransform);
        assert_eq!(p.ambient_scale, 0.5);
        let g = grading_for(p);
        assert!((g.global.post_saturation - 1.1).abs() < 1e-6);
    }

    #[test]
    fn ambient_scale_fades_out_at_dusk_and_night() {
        assert_eq!(effective_ambient_scale(0.5, 60.0), 0.5);
        assert_eq!(effective_ambient_scale(0.5, 25.0), 0.5);
        assert_eq!(effective_ambient_scale(0.5, 0.0), 1.0);
        assert_eq!(effective_ambient_scale(0.5, -30.0), 1.0);
        let dusk = effective_ambient_scale(0.5, 8.0);
        assert!(dusk > 0.5 && dusk < 1.0);
        assert_eq!(
            effective_ambient_scale(1.0, 40.0),
            1.0,
            "Bevy default never changes it"
        );
    }

    #[test]
    fn the_hdr_clear_colour_round_trips_through_the_tonemapper() {
        for display in [
            [0.5, 0.6, 0.8],
            [0.30, 0.35, 0.45],
            [0.8, 0.8, 0.8],
            [0.0, 0.0, 0.0],
        ] {
            let x = hdr_clear_color(Tonemapping::SomewhatBoringDisplayTransform, display);
            let back = somewhat_boring(x);
            for i in 0..3 {
                assert!(
                    (back[i] - display[i]).abs() < 0.01,
                    "{display:?} -> {x:?} -> {back:?}"
                );
            }
            let r = hdr_clear_color(Tonemapping::Reinhard, display);
            for i in 0..3 {
                assert!((reinhard(r[i]) - display[i]).abs() < 0.01);
            }
        }
        assert_eq!(
            hdr_clear_color(Tonemapping::None, [0.2, 0.4, 0.6]),
            [0.2, 0.4, 0.6]
        );
    }

    fn app_with_look(look: Look) -> App {
        let mut app = App::new();
        app.insert_resource(ActiveLook(look));
        app.add_plugins(LookPlugin);
        app
    }

    #[test]
    fn cameras_follow_the_active_look_live() {
        let mut app = app_with_look(Look::Vivid);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), opts_bevy::MainCamera))
            .id();
        app.update();
        let tm = *app.world().get::<Tonemapping>(cam).expect("tonemapping");
        assert_eq!(tm, Tonemapping::SomewhatBoringDisplayTransform);
        let g = app.world().get::<ColorGrading>(cam).expect("grading");
        assert!((g.global.post_saturation - 1.1).abs() < 1e-6);

        app.world_mut().resource_mut::<ActiveLook>().0 = Look::BevyDefault;
        app.update();
        assert_eq!(
            *app.world().get::<Tonemapping>(cam).expect("tonemapping"),
            Tonemapping::default()
        );
        assert_eq!(
            *app.world().get::<ColorGrading>(cam).expect("grading"),
            ColorGrading::default()
        );
        // A camera spawned later gets the current look although the look did not change.
        let late = app
            .world_mut()
            .spawn((Camera3d::default(), opts_bevy::MainCamera))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Tonemapping>(late).expect("tonemapping"),
            Tonemapping::default()
        );
    }
}
