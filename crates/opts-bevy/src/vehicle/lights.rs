//! The lights of the vehicle: interior lights, headlight placement and the spotlight children.

use super::*;

pub(super) fn spawn_interior_light_child(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    light: &InteriorLight,
) {
    let pos_bevy = omsi_to_bevy_pos(light.pos);
    let color = Color::srgb_u8(light.color_rgb[0], light.color_rgb[1], light.color_rgb[2]);

    let point = PointLight {
        color,
        range: light.range,
        radius: 0.1,
        shadow_maps_enabled: false,
        ..default()
    };
    let switch = ScriptLight {
        owner: parent,
        switch: LightSwitch::Variable(light.variable.clone()),
        base_intensity: point.intensity,
    };
    // Cabin lights are on their own render layer: the main camera only includes it in an
    // interior view or close to the vehicle (`light_rooms`). Lights only, no meshes.
    let child = commands
        .spawn((
            point,
            switch,
            Transform::from_translation(pos_bevy),
            RenderLayers::layer(crate::light_rooms::CABIN_LAYER),
        ))
        .insert(Name::new(format!(
            "InteriorLight_{index}_{}",
            light.variable
        )))
        .id();

    commands.entity(parent).add_children(&[child]);
}

/// Where the headlights of a model are along the vehicle (OMSI +y): the front-most forward-facing
/// white `[light_enh_2]`; `None` when the model has none.
#[must_use]
pub(super) fn headlight_front_y(model: &ModelConfig) -> Option<f32> {
    crate::vehicle_glow::detail_lights(model)
        .filter(|l| crate::vehicle_glow::is_headlight(l))
        .map(|l| l.position.y)
        .reduce(f32::max)
}

/// The place a `[spotlight]` shines from. Some models (the MAN Lion's City) put the emitters of
/// the beams 3 m behind the front, inside the body behind the driver's cabin: OMSI does not mind
/// (nothing casts shadows there), but here the beam's shadow map fills the passenger area with hard
/// shadows and a flooded floor. A forward-pointing emitter behind the headlights is moved forward
/// to them.
#[must_use]
pub(super) fn spot_emitter(spot: &Spotlight, front_y: Option<f32>) -> Vec3 {
    let mut pos = spot.pos;
    if let Some(front) = front_y {
        if spot.dir.y > 0.5 && pos.y < front - 0.5 {
            pos.y = front;
        }
    }
    pos
}

pub(super) fn spawn_spotlight_child(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    spot: &Spotlight,
    front_y: Option<f32>,
) {
    let pos_bevy = omsi_to_bevy_pos(spot_emitter(spot, front_y));
    let dir_bevy = omsi_to_bevy_pos(spot.dir).normalize_or_zero();
    let rotation = if dir_bevy.length_squared() > 1e-4 {
        Quat::from_rotation_arc(Vec3::NEG_Z, dir_bevy)
    } else {
        Quat::IDENTITY
    };

    let color = Color::srgb_u8(spot.color_rgb[0], spot.color_rgb[1], spot.color_rgb[2]);

    let light = SpotLight {
        color,
        range: spot.range,
        inner_angle: spot.inner_cone.to_radians(),
        outer_angle: spot.outer_cone.to_radians(),
        shadow_maps_enabled: true,
        ..default()
    };
    let switch = ScriptLight {
        owner: parent,
        switch: LightSwitch::Spot(index),
        base_intensity: light.intensity,
    };
    let child = commands
        .spawn((
            light,
            switch,
            Transform {
                translation: pos_bevy,
                rotation,
                scale: Vec3::ONE,
            },
        ))
        .insert(Name::new(format!("Spotlight_{index}")))
        .id();
    // The visible beam: a child of the light, so it points where the light points. Its
    // brightness is the light's own (`Spot_Select`).
    crate::light_cones::spawn_light_cone(
        commands,
        child,
        crate::light_cones::LightCone {
            kind: crate::light_cones::ConeKind::Headlight,
            shape: crate::light_cones::spotlight_cone_shape(spot),
            level: crate::light_cones::ConeLevel::Spot(child),
        },
    );

    commands.entity(parent).add_children(&[child]);
}
