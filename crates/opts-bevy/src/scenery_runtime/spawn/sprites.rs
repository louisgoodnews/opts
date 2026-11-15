//! The light sprites of an object: one sprite per light effect under its parent.

use super::*;

/// Spawns the sprite of one light effect under `parent`.
pub(super) fn build_light(
    commands: &mut Commands,
    parent: Entity,
    light: &ScoLight,
    engine: Option<&mut ScriptEngine>,
    assets: &mut SceneryBuildAssets<'_>,
) -> Entity {
    let var = if light.variable.trim().is_empty() {
        VarRef::One
    } else {
        resolve(light.variable.trim(), engine)
    };
    // Cone angles are full angles (as for Direct3D spotlights) ❓.
    let half = |deg: f32| (deg.clamp(0.0, 360.0) * 0.5).to_radians().cos();
    let sprite = SceneryLight {
        var,
        factor: light.factor,
        size_m: light.size_m.max(0.01),
        direction: if light.omni {
            Vec3::ZERO
        } else {
            local(light.direction).normalize_or_zero()
        },
        cos_inner: half(light.cone_inner_deg),
        cos_outer: half(light.cone_outer_deg),
        level: 0.0,
    };
    let mesh = assets.lights.glow_mesh(assets.meshes);
    let material = assets.lights.material(assets.materials, light.color);
    let e = spawn_light_sprite(commands, sprite, local(light.position), mesh, material);
    commands.entity(parent).add_child(e);
    // A directional light with the `cone` flag also draws a visible cone, next to the glow; its
    // brightness is the glow's level (the same variable, switched by the same night rule).
    if let Some(shape) = crate::light_cones::lantern_cone_shape(light) {
        crate::light_cones::spawn_light_cone(
            commands,
            parent,
            crate::light_cones::LightCone {
                kind: crate::light_cones::ConeKind::Lantern,
                shape,
                level: crate::light_cones::ConeLevel::Glow(e),
            },
        );
    }
    e
}
