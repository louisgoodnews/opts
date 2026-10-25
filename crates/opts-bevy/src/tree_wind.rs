//! Trees move in the wind (M-0144; `docs/src/architecture/vegetation-wind.md`).
//!
//! OMSI's trees stand still; this is an OPTS extension. A `[tree]` billboard is spawned with its
//! ordinary material and the [`WindTree`] marker; [`adopt_tree_material`] gives it the
//! [`TreeMaterial`] (the same material extended by a vertex stage, `shaders/tree_wind.wesl`, that
//! bends the crown along the wind in the main pass and in the depth and shadow passes). Without
//! [`TreeWindPlugin`] (headless runs) trees keep their material and stand still. The wind is a
//! uniform of the material: [`update_tree_wind`] writes it when the weather or the setting
//! changes; the time comes from the shader globals.

use std::collections::HashMap;

use bevy::asset::{load_internal_asset, Asset};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::environment::SessionEnvironment;

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("6f707473-7472-6565-7769-6e6400000001");

/// Crown displacement of a 10 m tree in calm weather (m).
const CALM_M: f32 = 0.04;
/// Additional crown displacement per m/s of wind (m).
const PER_MS_M: f32 = 0.025;
/// The largest crown displacement of a 10 m tree (m).
const MAX_M: f32 = 0.6;
/// The slow sway's frequency (Hz).
const SWAY_HZ: f32 = 0.4;
/// The direction a calm day sways along (Bevy x, z).
const CALM_DIRECTION: Vec2 = Vec2::new(0.6, 0.8);

/// The wind of the tree material.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct TreeWind {
    /// x, y: the direction the wind blows towards (Bevy x and z); z: crown displacement in metres
    /// for a 10 m tree (0: still); w: sway frequency in Hz.
    #[uniform(100)]
    pub wind: Vec4,
}

impl MaterialExtension for TreeWind {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// The material of `[tree]` billboards.
pub type TreeMaterial = ExtendedMaterial<StandardMaterial, TreeWind>;

/// Whether trees move (`graphics.tree_wind`), and the wind they move in now.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct TreeWindState {
    /// The setting.
    pub enabled: bool,
    /// The uniform every tree material carries.
    pub wind: Vec4,
}

impl Default for TreeWindState {
    fn default() -> Self {
        Self {
            enabled: true,
            wind: wind_uniform(true, Vec2::ZERO, 0.0),
        }
    }
}

/// The uniform for a wind blowing towards `to` (unit vector on the ground, Bevy x and z) at
/// `speed` m/s; zero strength when the setting is off.
#[must_use]
pub fn wind_uniform(enabled: bool, to: Vec2, speed: f32) -> Vec4 {
    if !enabled {
        return Vec4::new(CALM_DIRECTION.x, CALM_DIRECTION.y, 0.0, SWAY_HZ);
    }
    let dir = if speed > 0.0 {
        to.try_normalize().unwrap_or(CALM_DIRECTION)
    } else {
        CALM_DIRECTION
    };
    let strength = (CALM_M + PER_MS_M * speed.max(0.0)).min(MAX_M);
    Vec4::new(dir.x, dir.y, strength, SWAY_HZ)
}

/// Marks a tree whose material should move in the wind.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WindTree;

/// Gives new [`WindTree`]s the tree material: their own material, extended by the wind (one tree
/// material per base material, so trees of one texture share it).
pub fn adopt_tree_material(
    mut commands: Commands,
    trees: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Added<WindTree>>,
    standard: Res<Assets<StandardMaterial>>,
    mut tree_materials: ResMut<Assets<TreeMaterial>>,
    state: Res<TreeWindState>,
    mut adopted: Local<HashMap<AssetId<StandardMaterial>, Handle<TreeMaterial>>>,
) {
    for (entity, material) in &trees {
        let handle = if let Some(h) = adopted.get(&material.0.id()) {
            h.clone()
        } else {
            let Some(base) = standard.get(&material.0) else {
                continue;
            };
            let h = tree_materials.add(TreeMaterial {
                base: base.clone(),
                extension: TreeWind { wind: state.wind },
            });
            adopted.insert(material.0.id(), h.clone());
            h
        };
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

/// Follows the weather's wind and the setting: when the wind uniform changes, every tree material
/// gets it (one material per tree texture, so a handful).
pub fn update_tree_wind(
    env: Option<Res<SessionEnvironment>>,
    mut state: ResMut<TreeWindState>,
    mut materials: ResMut<Assets<TreeMaterial>>,
) {
    let (to, speed) = env.map_or((Vec2::ZERO, 0.0), |e| {
        let c = e.conditions();
        (c.wind_to(), c.wind_speed)
    });
    let wind = wind_uniform(state.enabled, to, speed);
    if wind == state.wind {
        return;
    }
    state.wind = wind;
    tracing::debug!(
        ?wind,
        speed,
        materials = materials.len(),
        "tree wind changed"
    );
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(mut m) = materials.get_mut(id) {
            m.extension.wind = wind;
        }
    }
}

/// Registers the tree material, its shader and the wind update.
#[derive(Default)]
pub struct TreeWindPlugin;

impl Plugin for TreeWindPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "shaders/tree_wind.wesl", Shader::from_wesl);
        app.add_plugins(MaterialPlugin::<TreeMaterial>::default())
            .init_resource::<TreeWindState>()
            .add_systems(Update, (update_tree_wind, adopt_tree_material).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wind_follows_the_weather_and_the_setting() {
        let calm = wind_uniform(true, Vec2::new(1.0, 0.0), 0.0);
        assert!(
            (calm.z - CALM_M).abs() < 1e-6,
            "a calm day still moves a little"
        );
        assert_eq!(Vec2::new(calm.x, calm.y), CALM_DIRECTION);
        let breeze = wind_uniform(true, Vec2::new(0.0, -2.0), 5.0);
        assert_eq!(Vec2::new(breeze.x, breeze.y), Vec2::new(0.0, -1.0));
        assert!(breeze.z > calm.z);
        let storm = wind_uniform(true, Vec2::X, 18.9);
        assert!((storm.z - 0.5125).abs() < 1e-4, "{}", storm.z);
        assert!(wind_uniform(true, Vec2::X, 100.0).z <= MAX_M);
        assert_eq!(wind_uniform(false, Vec2::X, 18.9).z, 0.0, "off: still");
    }

    #[test]
    fn a_changed_wind_reaches_every_tree_material() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .init_asset::<TreeMaterial>()
            .init_resource::<TreeWindState>()
            .add_systems(Update, update_tree_wind);
        let state = *app.world().resource::<TreeWindState>();
        let a = app
            .world_mut()
            .resource_mut::<Assets<TreeMaterial>>()
            .add(TreeMaterial {
                base: crate::tree_billboard_material(Handle::default()),
                extension: TreeWind { wind: state.wind },
            });
        app.update();
        app.world_mut().resource_mut::<TreeWindState>().enabled = false;
        app.update();
        let m = app
            .world()
            .resource::<Assets<TreeMaterial>>()
            .get(&a)
            .unwrap();
        assert_eq!(m.extension.wind.z, 0.0);
        app.world_mut().resource_mut::<TreeWindState>().enabled = true;
        app.update();
        let m = app
            .world()
            .resource::<Assets<TreeMaterial>>()
            .get(&a)
            .unwrap();
        assert!(m.extension.wind.z > 0.0);
    }

    #[test]
    fn a_marked_tree_gets_the_tree_material_shared_per_texture() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_asset::<TreeMaterial>()
            .init_resource::<TreeWindState>()
            .add_systems(Update, adopt_tree_material);
        let base = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(crate::tree_billboard_material(Handle::default()));
        let trees: Vec<Entity> = (0..3)
            .map(|_| {
                app.world_mut()
                    .spawn((WindTree, MeshMaterial3d(base.clone())))
                    .id()
            })
            .collect();
        let plain = app.world_mut().spawn(MeshMaterial3d(base.clone())).id();
        app.update();
        let handles: Vec<_> = trees
            .iter()
            .map(|e| {
                assert!(app
                    .world()
                    .get::<MeshMaterial3d<StandardMaterial>>(*e)
                    .is_none());
                app.world()
                    .get::<MeshMaterial3d<TreeMaterial>>(*e)
                    .expect("tree material")
                    .0
                    .id()
            })
            .collect();
        assert!(
            handles.windows(2).all(|w| w[0] == w[1]),
            "one material per texture"
        );
        assert!(app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(plain)
            .is_some());
        assert_eq!(app.world().resource::<Assets<TreeMaterial>>().len(), 1);
    }
}
