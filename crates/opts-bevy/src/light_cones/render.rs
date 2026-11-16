//! Drawing the cones: style, material, shared assets, the unit cone mesh and the render plugin.

use super::*;

mod assets;
mod material;
mod mesh;

pub use assets::*;
pub use material::*;
pub use mesh::*;


/// The cone material, its shader and the systems that give cones their mesh. Needs the render
/// app; the game adds it next to the ground material.
#[derive(Default)]
pub struct LightConeRenderPlugin;

impl Plugin for LightConeRenderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "../shaders/light_cone.wesl", Shader::from_wesl);
        if !app.is_plugin_added::<LightConePlugin>() {
            app.add_plugins(LightConePlugin);
        }
        app.add_plugins(MaterialPlugin::<LightConeMaterial>::default())
            .init_resource::<ConeStyle>()
            .init_resource::<ConeAssets>()
            .add_systems(Update, (attach_cone_assets, sync_cone_style));
    }
}
