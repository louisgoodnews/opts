//! The cone assets: the shared material and mesh, and the systems that attach them and sync the style.

use super::*;

/// The shared cone mesh and material.
#[derive(Resource, Debug, Clone)]
pub struct ConeAssets {
    /// The unit cone.
    pub mesh: Handle<Mesh>,
    /// The cone material.
    pub material: Handle<LightConeMaterial>,
}

impl FromWorld for ConeAssets {
    fn from_world(world: &mut World) -> Self {
        let style = world
            .get_resource::<ConeStyle>()
            .copied()
            .unwrap_or_default();
        let mesh = world.resource_mut::<Assets<Mesh>>().add(unit_cone_mesh());
        let material = world
            .resource_mut::<Assets<LightConeMaterial>>()
            .add(LightConeMaterial {
                params: style.uniform(),
            });
        Self { mesh, material }
    }
}

/// Gives new cones their mesh and material.
pub(super) fn attach_cone_assets(
    mut commands: Commands,
    assets: Res<ConeAssets>,
    fresh: Query<Entity, (With<LightCone>, Without<Mesh3d>)>,
) {
    for e in &fresh {
        commands.entity(e).insert((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
        ));
    }
}

/// Pushes a changed [`ConeStyle`] into the shared material.
pub(super) fn sync_cone_style(
    style: Res<ConeStyle>,
    assets: Res<ConeAssets>,
    mut materials: ResMut<Assets<LightConeMaterial>>,
) {
    if !style.is_changed() {
        return;
    }
    if let Some(mut m) = materials.get_mut(&assets.material) {
        m.params = style.uniform();
    }
}
