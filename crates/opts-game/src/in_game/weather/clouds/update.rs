//! The per-frame update of the cloud layer: moving, texturing and removing the disc.

use bevy::prelude::*;

use super::*;

/// Spawns, moves, textures and removes the cloud disc.
pub fn update_clouds(
    scene: super::super::WeatherScene,
    assets: super::super::WeatherAssets,
    inputs: CloudInputs,
    mut state: ResMut<CloudState>,
    mut commands: Commands,
) {
    let super::super::WeatherScene { time, env, effects } = scene;
    let super::super::WeatherAssets {
        mut images,
        mut meshes,
        mut materials,
    } = assets;
    let CloudInputs {
        weather,
        cameras,
        mut planes,
    } = inputs;
    let conditions = env.conditions();
    let layer = conditions
        .cloud
        .as_ref()
        .filter(|_| effects.clouds)
        .and_then(|c| c.texture.as_ref().map(|t| (c, t.clone())));
    let Some((cloud, file)) = layer else {
        for (e, ..) in &planes {
            commands.entity(e).despawn();
        }
        return;
    };
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let handle = state
        .textures
        .entry(file.clone())
        .or_insert_with(|| load_texture(&weather, &file).map(|i| images.add(i)))
        .clone();
    let Some(handle) = handle else {
        for (e, ..) in &planes {
            commands.entity(e).despawn();
        }
        return;
    };
    let dt = time.delta_secs();
    let reference = state.reference_y.get_or_insert(cam.y);
    *reference += (cam.y - *reference) * (1.0 - (-dt * HEIGHT_FOLLOW).exp());
    let y = *reference + cloud.base_m;
    let tint = weather_look(conditions, [0.0; 3], env.sun_alt_deg()).cloud;
    let colour = Color::linear_rgb(tint, tint, tint);
    let transform = uv_transform(
        Vec2::new(cam.x, cam.z),
        cloud_drift(conditions),
        time.elapsed_secs_f64(),
        cloud.size_m,
    );

    let Some((_, mut t, material)) = planes.iter_mut().next() else {
        spawn_plane(
            &mut commands,
            (&mut meshes, &mut materials),
            &mut state,
            (handle, file),
            (colour, tint, transform),
            Vec3::new(cam.x, y, cam.z),
        );
        return;
    };
    t.translation = Vec3::new(cam.x, y, cam.z);
    if let Some(mut m) = materials.get_mut(&material.0) {
        m.uv_transform = transform;
        if state.shown.as_deref() != Some(file.as_str()) {
            m.base_color_texture = Some(handle);
            state.shown = Some(file);
        }
        if (state.tint - tint).abs() > 0.004 {
            m.base_color = colour;
            state.tint = tint;
        }
    };
}
