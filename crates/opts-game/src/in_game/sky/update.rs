//! The per-frame update of the sky: the dome, the texture, the discs and the lighting.

use bevy::prelude::*;

use super::*;

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
pub(super) type CameraFilter = (With<opts_bevy::MainCamera>, Without<SkyDome>);
pub(super) type SunFilter = (With<SunLight>, Without<SkyDome>);
pub(super) type DiscFilter = (
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
