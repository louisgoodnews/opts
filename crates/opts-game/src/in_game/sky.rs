//! Data-driven sky: dome texture, sun/moon discs and light colours from the installation's
//! `envir.cfg`, driven by the sun of the [`SessionEnvironment`] (date, time, weather).
//!
//! OMSI maps carry no geographic position, so the sun path assumes a fixed mid-latitude
//! (`opts_bevy::environment::LATITUDE_DEG`) and treats the clock time as local solar time.

use bevy::prelude::*;
use opts_bevy::ground::CpuImage;
use opts_bevy::SessionEnvironment;
use opts_core::formats::envir::EnvirConfig;

/// Radius of the sky dome; stays inside the default 1000 m far plane.
pub(crate) const DOME_RADIUS: f32 = 900.0;
/// Clock seconds between sky texture rebuilds (a jump of the environment rebuilds at once).
const REFRESH_S: f64 = 0.5;
/// Peak illuminance of full white direct sunlight.
const SUN_ILLUMINANCE: f32 = 20_000.0;
/// Ambient brightness for full white ambient light.
const AMBIENT_BRIGHTNESS: f32 = 700.0;

/// Sky state for the running session.
#[derive(Resource)]
pub struct Sky {
    pub(super) config: EnvirConfig,
    pub(super) textures: [CpuImage; 3],
    pub(super) image: Handle<Image>,
    /// Clock seconds of the last rebuild.
    pub(super) last_refresh: f64,
    /// Environment revision of the last rebuild (`None` before the first).
    pub(super) last_revision: Option<u64>,
    /// Whether the last rebuild wrote the clear colour for an HDR camera.
    pub(super) last_hdr: Option<bool>,
    /// The ground-fog sky clearing of the last rebuild, in 1/32 steps (the sky is rebuilt when
    /// the camera's height moves it by a step).
    pub(super) last_fog_clear: u8,
}

/// The sky's colour at the horizon (linear RGB `0..=1`), refreshed with the sky texture; the fog
/// takes its colour from it (`weather::fog`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SkyHorizon(pub [f32; 3]);

/// Marks the sky dome.
#[derive(Component)]
pub struct SkyDome;
/// Marks the sun light and disc.
#[derive(Component)]
pub struct SunLight;
/// Marks the sun disc quad.
#[derive(Component)]
pub struct SunDisc;
/// Marks the moon disc quad; `in_game::moon` gives it its mesh and material.
#[derive(Component)]
pub struct MoonDisc;
/// The moon photograph (`Texture/mond.bmp`) waiting for the disc's material.
#[derive(Component)]
pub struct MoonDiscTexture(pub Option<Handle<Image>>);

mod spawn;
pub use spawn::*;

/// Sun elevation and azimuth (degrees; azimuth clockwise from north) for the local solar hour:
/// [`opts_bevy::environment::sun_position`] at the sky's latitude.
#[must_use]
pub fn sun_position(hour: f32, day_of_year: f32) -> (f32, f32) {
    opts_bevy::environment::sun_position(hour, day_of_year, opts_bevy::environment::LATITUDE_DEG)
}

/// Direction towards a point at the given elevation/azimuth, in Bevy space (north = −Z).
pub(crate) fn direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (el, az) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(el.cos() * az.sin(), el.sin(), -el.cos() * az.cos())
}

/// Moves the sky dome to the [`MainCamera`](opts_bevy::MainCamera) and turns it so that texture
/// `u = 0` faces the sun (the dusk texture's glow is authored around it).
pub fn place_sky_dome(
    env: Res<SessionEnvironment>,
    cameras: Query<&GlobalTransform, (With<opts_bevy::MainCamera>, Without<SkyDome>)>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    for mut t in &mut dome {
        t.translation = cam;
        t.rotation = Quat::from_rotation_y(-env.sun_az_deg().to_radians());
    }
}

/// The colour of the sky at the horizon (the last row of the textures, blended by `weights`); it
/// fills anything the dome does not cover and the fog takes it too.
fn horizon_colour(sky: &Sky, weights: [f32; 3]) -> [f32; 3] {
    let h = sky.textures[0].height - 1;
    let mut horizon = [0.0; 3];
    for (ch, out) in horizon.iter_mut().enumerate() {
        for (t, wt) in sky.textures.iter().zip(weights) {
            *out += f32::from(t.rgba[(h * t.width) * 4 + ch]) * wt / 255.0;
        }
    }
    horizon
}

/// The sun's colour and strength from the sky's light curve at the sun elevation, scaled by the
/// weather (`opts_bevy::weather::light_scales`).
fn sun_light(sky: &Sky, env: &SessionEnvironment, elevation: f32) -> (Color, f32) {
    let a = sky.config.light_at(&sky.config.light_a, elevation);
    let mean = (a[0] + a[1] + a[2]) / 3.0;
    let att = env.light_attenuation().direct;
    (
        Color::srgb(a[0].max(0.001), a[1].max(0.001), a[2].max(0.001)),
        SUN_ILLUMINANCE * mean * att,
    )
}

/// The ambient light from the sky's light curves at the sun elevation, scaled by the weather.
fn set_ambient(
    sky: &Sky,
    ambient: &mut GlobalAmbientLight,
    env: &SessionEnvironment,
    (elevation, ambient_scale): (f32, f32),
) {
    let b = sky.config.light_at(&sky.config.light_b, elevation);
    let c = sky.config.light_at(&sky.config.light_c, elevation);
    let amb = [c[0] + b[0] * 0.5, c[1] + b[1] * 0.5, c[2] + b[2] * 0.5];
    let peak = amb.iter().copied().fold(0.001_f32, f32::max);
    ambient.color = Color::srgb(amb[0] / peak, amb[1] / peak, amb[2] / peak);
    ambient.brightness =
        AMBIENT_BRIGHTNESS * ambient_scale * peak.min(1.0) * env.light_attenuation().ambient;
}

/// Writes the clear colour; with HDR what shows the colour after the camera's tonemapping.
fn set_clear_colour(
    clear: &mut ClearColor,
    look: Option<&crate::look::ActiveLook>,
    hdr: bool,
    clear_color: [f32; 3],
) {
    let display = Color::srgb(clear_color[0], clear_color[1], clear_color[2]).to_linear();
    clear.0 = if hdr {
        // The HDR camera tonemaps its clear colour too; write what shows `display` after it.
        let tm = look
            .as_ref()
            .map_or_else(Default::default, |l| l.0.params().tonemapping);
        let x = crate::look::hdr_clear_color(tm, [display.red, display.green, display.blue]);
        Color::linear_rgb(x[0], x[1], x[2])
    } else {
        display.into()
    };
}

/// Rebuilds the sky texture (the blend of the three by the sun's elevation, veiled by fog) and
/// returns the colour at the horizon and the fog's veil colour.
fn refresh_texture(
    sky: &Sky,
    env: &SessionEnvironment,
    (fog_on, fog_clear, elevation): (bool, u8, f32),
    images: &mut Assets<Image>,
) -> ([f32; 3], [f32; 3]) {
    let weights = sky.config.sky_weights(elevation);
    // Colour of the sky at the horizon fills anything the dome does not cover; the fog
    // takes it too, greyed in thick fog.
    let horizon = horizon_colour(sky, weights);
    let conditions = env.conditions();
    let veil_color = opts_bevy::weather::fog_color(horizon, conditions);
    let data = blended_sky(
        sky,
        weights,
        |v| {
            if fog_on {
                opts_bevy::weather::sky_veil_clear(conditions, v, f32::from(fog_clear) / 32.0)
            } else {
                0.0
            }
        },
        veil_color,
    );
    if let Some(mut img) = images.get_mut(&sky.image) {
        img.data = Some(data);
    }
    (horizon, veil_color)
}

/// The clear colour fills what the dome does not cover: the fog colour, towards the horizon
/// colour as the camera leaves the ground-fog layer.
fn clear_colour_of(
    horizon: [f32; 3],
    veil_color: [f32; 3],
    fog_on: bool,
    fog_clear: u8,
) -> [f32; 3] {
    let lift = f32::from(fog_clear) / 32.0;

    if fog_on {
        [0, 1, 2].map(|c| veil_color[c] + (horizon[c] - veil_color[c]) * lift)
    } else {
        horizon
    }
}

/// The sun disc: on the dome in the sun's direction, hidden below the horizon.
fn place_on_dome(cam: Vec3, dir: Vec3) -> Transform {
    let mut t = Transform::from_translation(cam + dir * (DOME_RADIUS - 10.0));
    t.look_at(cam, Vec3::Y);
    if dir.y <= -0.05 {
        t.scale = Vec3::ZERO;
    }
    t
}

/// Optional read-only inputs of [`update_sky`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct SkyInputs<'w> {
    effects: Option<Res<'w, opts_bevy::WeatherEffects>>,
    fog_view: Option<Res<'w, opts_bevy::weather::GroundFogView>>,
    look: Option<Res<'w, crate::look::ActiveLook>>,
    settings: Option<Res<'w, crate::settings::GameSettings>>,
}

/// What [`update_sky`] repaints: the sky texture, the clear colour and the ambient light.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SkyTargets<'w> {
    images: ResMut<'w, Assets<Image>>,
    clear: Option<ResMut<'w, ClearColor>>,
    ambient: Option<ResMut<'w, GlobalAmbientLight>>,
}

/// The queries of [`update_sky`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct SkyQueries<'w, 's> {
    cameras: Query<'w, 's, &'static GlobalTransform, CameraFilter>,
    sun: Query<'w, 's, (&'static mut Transform, &'static mut DirectionalLight), SunFilter>,
    sun_disc: Query<'w, 's, &'static mut Transform, DiscFilter>,
}
type CameraFilter = (With<opts_bevy::MainCamera>, Without<SkyDome>);
type SunFilter = (With<SunLight>, Without<SkyDome>);
type DiscFilter = (
    With<SunDisc>,
    Without<SkyDome>,
    Without<SunLight>,
    Without<MoonDisc>,
);

/// Updates sky colours, sun/moon placement and lighting.
pub fn update_sky(
    env: Res<SessionEnvironment>,
    mut commands: Commands,
    sky: Option<ResMut<Sky>>,
    inputs: SkyInputs,
    mut targets: SkyTargets,
    mut queries: SkyQueries,
) {
    let SkyInputs {
        effects,
        fog_view,
        look,
        settings,
    } = inputs;
    let SkyTargets {
        images,
        clear,
        ambient,
    } = &mut targets;
    let (cameras, sun, sun_disc) = (&queries.cameras, &mut queries.sun, &mut queries.sun_disc);
    let Some(mut sky) = sky else { return };
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    // The session environment (not the app's virtual time, which keeps running between
    // sessions), so a restored save and a changed time show their sky at once.
    let now = env.seconds_of_day();
    let (elevation, azimuth) = (env.sun_alt_deg(), env.sun_az_deg());
    let sun_dir = direction(elevation, azimuth);

    // A new look changes the ambient scale: refresh at once instead of at the next interval.
    let look_changed = look.as_ref().is_some_and(|l| l.is_changed());
    let ambient_scale = look.as_ref().map_or(1.0, |l| {
        crate::look::effective_ambient_scale(l.0.params().ambient_scale, elevation)
    });
    // Switching `graphics.hdr` changes how the clear colour must be written.
    let hdr = settings
        .as_ref()
        .is_some_and(|s| crate::settings_registry::live::hdr_enabled(s));
    let hdr_changed = sky.last_hdr != Some(hdr);
    let jumped = sky.last_revision != Some(env.revision());
    let effects_changed = effects.as_ref().is_some_and(|e| e.is_changed());
    // Above a ground-fog layer the sky is not veiled (`ground_fog`); quantised so the sky is not
    // rebuilt every frame while the camera climbs.
    let fog_clear = fog_view
        .as_ref()
        .map_or(0, |v| (v.sky_clear * 32.0).round() as u8);
    let fog_clear_changed = sky.last_fog_clear != fog_clear;
    if look_changed
        || fog_clear_changed
        || hdr_changed
        || jumped
        || effects_changed
        || (now - sky.last_refresh).abs() >= REFRESH_S
    {
        sky.last_refresh = now;
        sky.last_hdr = Some(hdr);
        sky.last_fog_clear = fog_clear;
        sky.last_revision = Some(env.revision());
        let fog_on = effects.as_ref().is_none_or(|e| e.fog);
        let (horizon, veil_color) =
            refresh_texture(&sky, &env, (fog_on, fog_clear, elevation), images);
        let (sun_colour, sun_lux) = sun_light(&sky, &env, elevation);
        for (_, mut light) in sun.iter_mut() {
            light.color = sun_colour;
            light.illuminance = sun_lux;
        }
        if let Some(ambient) = ambient.as_mut() {
            set_ambient(&sky, ambient, &env, (elevation, ambient_scale));
        }
        let clear_color = clear_colour_of(horizon, veil_color, fog_on, fog_clear);
        if let Some(clear) = clear.as_mut() {
            set_clear_colour(clear, look.as_deref(), hdr, clear_color);
        }
        commands.insert_resource(SkyHorizon(horizon));
    }
    for (mut t, _) in sun.iter_mut() {
        *t = Transform::from_translation(cam).looking_to(-sun_dir, Vec3::Y);
    }
    for mut t in sun_disc.iter_mut() {
        *t = place_on_dome(cam, sun_dir);
    }
}

#[cfg(test)]
mod tests;
