//! The moon disc material: the moon photograph with the terminator of the current phase
//! (`docs/src/architecture/lighting-system.md` section 5.5), drawn additively and cut out with a
//! circle because `Texture/mond.bmp` has no alpha and a light background.

use bevy::asset::{load_internal_asset, Asset};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("6f707473-6d6f-6f6e-6469-736300000001");

/// Brightness of the unlit part of the disc (earthshine), `0..=1`.
pub const EARTHSHINE: f32 = 0.03;

/// The moon disc material.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MoonMaterial {
    /// `x` phase angle (radians, 0 full), `y` bright-limb direction (radians, counter-clockwise
    /// from the disc's right), `z` brightness, `w` earthshine.
    #[uniform(0)]
    pub params: Vec4,
    /// The moon photograph.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl MoonMaterial {
    /// A material for a phase angle and limb direction in degrees and a brightness.
    #[must_use]
    pub fn new(
        texture: Option<Handle<Image>>,
        phase_angle_deg: f32,
        bright_limb_deg: f32,
        brightness: f32,
    ) -> Self {
        Self {
            params: Vec4::new(
                phase_angle_deg.to_radians(),
                bright_limb_deg.to_radians(),
                brightness,
                EARTHSHINE,
            ),
            texture,
        }
    }
}

impl Material for MoonMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Registers the shader and the material type.
#[derive(Default)]
pub struct MoonMaterialPlugin;

impl Plugin for MoonMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "shaders/moon_disc.wesl", Shader::from_wesl);
        app.add_plugins(MaterialPlugin::<MoonMaterial>::default());
    }
}
