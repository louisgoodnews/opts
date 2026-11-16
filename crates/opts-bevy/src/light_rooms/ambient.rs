//! The interior ambient light: the light that lifts the cabin while the player is inside.

use super::*;


/// Marks the camera whose [`AmbientLight`] [`interior_ambient_system`] wrote.
#[derive(Component)]
pub struct InteriorAmbient;

/// In an interior view the main camera gets its own, dimmer ambient light (a camera's
/// `AmbientLight` replaces the global one); outside it is taken off again.
pub fn interior_ambient_system(
    mut commands: Commands,
    view: Res<CabinView>,
    global: Option<Res<GlobalAmbientLight>>,
    cameras: Query<(Entity, Has<InteriorAmbient>), With<crate::MainCamera>>,
) {
    for (camera, marked) in &cameras {
        match (view.interior_view, global.as_deref()) {
            (true, Some(g)) => {
                commands.entity(camera).insert((
                    InteriorAmbient,
                    AmbientLight {
                        color: g.color,
                        brightness: g.brightness * INTERIOR_AMBIENT_SCALE,
                        affects_lightmapped_meshes: g.affects_lightmapped_meshes,
                    },
                ));
            }
            _ if marked => {
                commands
                    .entity(camera)
                    .remove::<(InteriorAmbient, AmbientLight)>();
            }
            _ => {}
        }
    }
}
