//! The AI pools, the AI train and the spawn inputs: the records the AI traffic keeps.

use super::*;

/// Rail vehicles found in the map's AI lists.
#[derive(Resource, Default)]
pub struct RailAiPool {
    pub(in super::super) started: bool,
    pub(in super::super) specs: Vec<TrainSpec>,
    pub(in super::super) elapsed: f32,
    pub(in super::super) rng: u32,
    /// The vehicle files of every list group and depot group, by lower-case group name (the
    /// `vehicle_group` of a timetable tour).
    pub(in super::super) group_vehicles: std::collections::HashMap<String, Vec<String>>,
    /// The vehicle files (lower case) that are rail vehicles.
    pub(in super::super) rail_files: std::collections::HashSet<String>,
}

/// Riders found in the map's AI lists (light `[type] 2` vehicles: bicycles, e-scooters).
#[derive(Resource, Default)]
pub struct BikeAiPool(pub(super) RailAiPool);

/// A running AI train.
#[derive(Component)]
pub struct AiTrain {
    pub(in super::super) bike: bool,
    pub(in super::super) train: Train,
    pub(in super::super) speed: f32,
    pub(in super::super) composition: VehicleComposition,
    /// Half bogie span of the head unit.
    pub(in super::super) wheel_angle: f32,
    /// The timetable run it drives, if it is a scheduled train.
    pub(in super::super) plan: Option<super::scheduled::RailPlan>,
}

/// The settings, networks and spec pools of AI trains and cyclists.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiFleet<'w> {
    pub(in super::super) settings: Res<'w, RailSettings>,
    pub(in super::super) bike_settings: Res<'w, BikeSettings>,
    pub(in super::super) net: Res<'w, RailNetwork>,
    pub(in super::super) bike_net: Res<'w, BikeNetwork>,
    pub(in super::super) pool: ResMut<'w, RailAiPool>,
    pub(in super::super) bike_pool: ResMut<'w, BikeAiPool>,
}

/// The asset stores the AI vehicles' meshes are added to.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiAssets<'w> {
    pub(in super::super) meshes: ResMut<'w, Assets<Mesh>>,
    pub(in super::super) materials: ResMut<'w, Assets<StandardMaterial>>,
    pub(in super::super) images: ResMut<'w, Assets<Image>>,
}

/// The canvases of a unit's script textures.
pub(in super::super) type ScriptCanvases =
    std::sync::Arc<std::sync::RwLock<Vec<opts_core::formats::script_texture::CpuScriptTexture>>>;

pub(in super::super) type SpawnAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);
