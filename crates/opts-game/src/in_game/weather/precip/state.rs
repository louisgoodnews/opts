//! The precipitation's state: the layer marker and the per-frame bookkeeping resource.

use bevy::prelude::*;
use opts_core::formats::weather::PrecipKind;

/// Marks the precipitation mesh.
#[derive(Component, Debug, Clone, Copy)]
pub struct PrecipLayer;

/// State of the precipitation.
#[derive(Resource, Default)]
pub struct PrecipState {
    /// Fixed random seeds `0..1` per particle (one per allocated quad).
    pub(super) seeds: Vec<[f32; 3]>,
    /// Indices of all allocated quads.
    pub(super) indices: Vec<u32>,
    pub(super) mesh: Option<Handle<Mesh>>,
    pub(super) rain: Option<Handle<StandardMaterial>>,
    pub(super) snow: Option<Handle<StandardMaterial>>,
    /// Quads the mesh has vertices for.
    pub(super) allocated: usize,
    /// Quads in the index buffer.
    pub(super) drawn: usize,
    /// Kind the layer shows.
    pub(super) kind: Option<PrecipKind>,
    /// Tint written to the materials last.
    pub(super) tint: f32,
}

impl PrecipState {
    /// Quads the mesh is built for (the budget of the last plan).
    #[must_use]
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// Quads that are drawn.
    #[must_use]
    pub fn drawn(&self) -> usize {
        self.drawn
    }

    /// The kind that is drawn, `None` while nothing falls.
    #[must_use]
    pub fn kind(&self) -> Option<PrecipKind> {
        self.kind
    }
}
