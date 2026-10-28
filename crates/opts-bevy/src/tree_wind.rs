//! Trees move in the wind (M-0144; `docs/src/architecture/vegetation-wind.md`).
//!
//! OMSI's trees stand still; this is an OPTS extension. A `[tree]` billboard is spawned with its
//! ordinary material and the [`WindTree`] marker; [`adopt_tree_material`] gives it the
//! [`TreeMaterial`] (the same material extended by a vertex stage, `shaders/tree_wind.wesl`, that
//! bends the crown along the wind in the main pass and in the depth and shadow passes). Without
//! [`TreeWindPlugin`] (headless runs) trees keep their material and stand still. The wind is a
//! A tree modelled as a mesh (its foliage cut-out cards in the `.o3d`) is marked with
//! [`MeshTreeRoot`]; [`tag_mesh_tree_foliage`] gives its non-opaque parts [`WindTree`] and
//! [`MeshTree`], whose material bends with the height above the object alone. The wind is a
//! uniform of the material: [`update_tree_wind`] writes it when the weather or the setting
//! changes; the time comes from the shader globals.

use std::collections::HashMap;

use bevy::asset::{load_internal_asset, Asset};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
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
///
/// Bindless like the `StandardMaterial` it extends: every tree material shares one bind group, so
/// trees of all textures are drawn in a few multi-draw calls. A plain `#[uniform]` made each tree
/// material its own bind group and draw batch (Waldhofen Bahnhof: 5,536 alpha-mask draw calls
/// instead of 7, 285 ms per frame instead of 41). Non-bindless devices read binding 50 as a
/// uniform; bindless ones the index table at 100 and the data array at 101.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
#[data(50, TreeWindUniform, binding_array(101))]
#[bindless(index_table(range(50..51), binding(100)))]
pub struct TreeWind {
    /// x, y: the direction the wind blows towards (Bevy x and z); z: crown displacement in metres
    /// for a 10 m tree (0: still); w: sway frequency in Hz.
    pub wind: Vec4,
    /// x: 1 for a `[tree]` billboard (the bend follows its texture's height, `1 - v`), 0 for a
    /// tree modelled as a mesh (the bend follows the height above the object alone; its foliage
    /// cards' texture coordinates say nothing about height).
    pub shape: Vec4,
}

/// The GPU data of [`TreeWind`].
#[derive(Clone, Default, ShaderType)]
pub struct TreeWindUniform {
    wind: Vec4,
    shape: Vec4,
}

impl From<&TreeWind> for TreeWindUniform {
    fn from(t: &TreeWind) -> Self {
        Self {
            wind: t.wind,
            shape: t.shape,
        }
    }
}

/// [`TreeWind::shape`] of a `[tree]` billboard.
pub const BILLBOARD: Vec4 = Vec4::new(1.0, 0.0, 0.0, 0.0);
/// [`TreeWind::shape`] of a mesh tree's foliage.
pub const MESH_TREE: Vec4 = Vec4::ZERO;

impl MaterialExtension for TreeWind {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// The material of `[tree]` billboards and of mesh trees' foliage.
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

/// With [`WindTree`]: the mesh is a modelled tree's foliage, not a billboard (M-0144 phase C).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MeshTree;

/// Marks the root of a placed object modelled as a tree (M-0144 phase C): [`tag_mesh_tree_foliage`]
/// lets its cut-out and blended parts (the foliage) move in the wind; the opaque trunk stays.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MeshTreeRoot;

/// Gives the foliage of new [`MeshTreeRoot`]s [`WindTree`] and [`MeshTree`]: every descendant
/// part whose material is not opaque.
pub fn tag_mesh_tree_foliage(
    mut commands: Commands,
    roots: Query<Entity, Added<MeshTreeRoot>>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<StandardMaterial>, Without<WindTree>>,
    standard: Res<Assets<StandardMaterial>>,
) {
    let mut tagged = 0usize;
    for root in &roots {
        for entity in children.iter_descendants(root) {
            let Ok(material) = parts.get(entity) else {
                continue;
            };
            if standard
                .get(&material.0)
                .is_some_and(|m| !matches!(m.alpha_mode, AlphaMode::Opaque))
            {
                commands.entity(entity).insert((WindTree, MeshTree));
                tagged += 1;
            }
        }
    }
    if tagged > 0 {
        tracing::debug!(tagged, "mesh tree foliage moves in the wind");
    }
}

/// New [`WindTree`]s with their material and whether they are a mesh tree's foliage.
type NewTrees<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MeshMaterial3d<StandardMaterial>,
        Has<MeshTree>,
    ),
    Added<WindTree>,
>;

/// The tree material made for a base material, per kind of tree (mesh foliage or not).
type AdoptedMaterials = HashMap<(AssetId<StandardMaterial>, bool), Handle<TreeMaterial>>;

/// Gives new [`WindTree`]s the tree material: their own material, extended by the wind (one tree
/// material per base material, so trees of one texture share it).
pub fn adopt_tree_material(
    mut commands: Commands,
    trees: NewTrees,
    standard: Res<Assets<StandardMaterial>>,
    mut tree_materials: ResMut<Assets<TreeMaterial>>,
    state: Res<TreeWindState>,
    mut adopted: Local<AdoptedMaterials>,
) {
    for (entity, material, mesh_tree) in &trees {
        let key = (material.0.id(), mesh_tree);
        let handle = if let Some(h) = adopted.get(&key) {
            h.clone()
        } else {
            let Some(base) = standard.get(&material.0) else {
                continue;
            };
            let h = tree_materials.add(TreeMaterial {
                base: base.clone(),
                extension: TreeWind {
                    wind: state.wind,
                    shape: if mesh_tree { MESH_TREE } else { BILLBOARD },
                },
            });
            adopted.insert(key, h.clone());
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
            .add_systems(
                Update,
                (tag_mesh_tree_foliage, update_tree_wind, adopt_tree_material).chain(),
            );
    }
}

#[cfg(test)]
mod tests;
