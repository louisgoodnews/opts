//! The root of a vehicle model configuration (`model.cfg`).

use super::*;
use std::collections::BTreeMap;

/// Root data model representing a vehicle rendering manifest (`model.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelConfig {
    /// Global texture detail scaling factor from `[tex_detail_factor]`.
    pub tex_detail_factor: Option<f32>,
    /// Color texture change schemes from `[CTC]`.
    pub ctc_schemes: Vec<CtcScheme>,
    /// Color texture change texture mappings from `[CTCTexture]`.
    pub ctc_textures: Vec<CtcTexture>,
    /// Dynamic text texture declarations.
    pub text_textures: Vec<TextTexture>,
    /// Dynamic script texture declarations.
    pub script_textures: Vec<ScriptTexture>,
    /// Level of detail tiers and mesh hierarchies.
    pub lods: Vec<ModelLod>,
    /// Unscoped interior lights.
    pub interior_lights: Vec<InteriorLight>,
    /// Unscoped spotlights.
    pub spotlights: Vec<Spotlight>,
    /// Light effects (`[light_enh]`, `[light_enh_2]`) in declaration order, whatever mesh or LOD
    /// group they follow (recorded in `after_mesh`/`lod_threshold`, the mesh index counted within
    /// its LOD group). Same layout as in a `.sco` ([`crate::formats::scenery::ScoLight`]).
    pub lights: Vec<crate::formats::scenery::ScoLight>,
    /// Blocks that were not understood (malformed light effects), with the reason.
    pub warnings: Vec<String>,
    /// Global unrecognized keywords.
    pub unknown_fields: BTreeMap<String, Vec<Vec<String>>>,
}

impl ModelConfig {
    /// Creates an empty model configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The meshes drawn at close range: those before the first `[LOD]` (always visible) together
    /// with the first explicit `[LOD]` group (the full-detail level). Later groups are simplified
    /// stand-ins for distance. A file whose first group is already an explicit `[LOD]` yields that
    /// group alone.
    ///
    /// The implicit group before the first marker is recognised by its threshold of 0 followed by
    /// an explicit group with a positive threshold (`[LOD] 0.07` ... `[LOD] 0` in the Aachen
    /// Citaro files).
    #[must_use]
    pub fn detail_lod(&self) -> Option<ModelLod> {
        let first = self.lods.first()?;
        match self.lods.get(1) {
            Some(second) if first.threshold <= 0.0 && second.threshold > 0.0 => {
                let mut merged = second.clone();
                merged.meshes.splice(0..0, first.meshes.iter().cloned());
                Some(merged)
            }
            _ => Some(first.clone()),
        }
    }

    /// Returns the total count of meshes across all LOD tiers.
    #[must_use]
    pub fn total_mesh_count(&self) -> usize {
        self.lods.iter().map(|lod| lod.meshes.len()).sum()
    }
}
