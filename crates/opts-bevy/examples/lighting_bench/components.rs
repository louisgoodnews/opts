//! Resources and marker components of the lighting benchmark, shared by the setup, drive and apply modules.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::*;

use super::config::Cfg;

#[derive(Resource)]
pub(super) struct Bench {
    pub(super) plan: Vec<Cfg>,
    pub(super) idx: usize,
    /// Frames since the current item was applied.
    pub(super) frame: u32,
    pub(super) ms: Vec<f32>,
    pub(super) gpu: BTreeMap<String, Vec<f64>>,
    pub(super) out: Option<PathBuf>,
    pub(super) shots: Option<PathBuf>,
    pub(super) started: std::time::Instant,
    pub(super) item_start: std::time::Instant,
    pub(super) shot_done: bool,
    pub(super) max_seconds: f32,
    pub(super) load: String,
}

#[derive(Component)]
pub(super) struct Cone;

#[derive(Component)]
pub(super) struct Sun;

#[derive(Component)]
pub(super) struct NightLamp(pub(super) usize);

#[derive(Component)]
pub(super) struct HeadSpot(pub(super) usize);

#[derive(Component)]
pub(super) struct Cam;

#[derive(Component)]
pub(super) struct Fog;

#[derive(Component)]
pub(super) struct Mirror;

#[derive(Resource, Default)]
pub(super) struct MirrorEvery(pub(super) u32);

#[derive(Resource)]
pub(super) struct Assets3 {
    pub(super) cone: Handle<Mesh>,
    pub(super) target: Option<Handle<Image>>,
    pub(super) quad: Handle<Mesh>,
}

#[derive(Resource)]
pub(super) struct WindowModeFlag(pub(super) bool);
