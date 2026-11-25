//! Mesh notes: how one mesh file was loaded, and the material slots it was built into.

use serde::{Deserialize, Serialize};

use super::geometry::TraceBounds;

/// How one mesh file was obtained and what it looks like; written once per file and run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MeshNote {
    /// File, relative to the OMSI folder.
    #[serde(default)]
    pub file: String,
    /// How the file became geometry: `plain`, `twin`, `decode-cache`, `decode-background`,
    /// `decoded-now`, `decode-deferred`, `rejected` (scrambled and not decodable), `extent`
    /// (implausible size), `read-failed`, `parse-failed`, `hidden` (every slot variable-hidden).
    pub stage: String,
    /// Time reading the file (ms).
    #[serde(default)]
    pub read_ms: f32,
    /// Time parsing (ms).
    #[serde(default)]
    pub parse_ms: f32,
    /// Time decoding a keyed mesh (ms).
    #[serde(default)]
    pub decode_ms: f32,
    /// Time building meshes, materials and textures (ms).
    #[serde(default)]
    pub build_ms: f32,
    /// Vertices of the file.
    #[serde(default)]
    pub vertices: usize,
    /// Parts (one per material slot) produced.
    #[serde(default)]
    pub parts: usize,
    /// Material slots with texture, alpha mode and result.
    #[serde(default)]
    pub materials: Vec<MaterialNote>,
    /// Warnings of the load.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Bounding box of the produced geometry (Bevy axes, the file's own space).
    #[serde(default)]
    pub bbox: Option<TraceBounds>,
}

/// One material slot of a mesh as it was built.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MaterialNote {
    /// Material slot of the `.o3d`.
    pub slot: usize,
    /// Texture reference as given (empty when none).
    #[serde(default)]
    pub texture: String,
    /// `opaque`, `mask` or `blend`.
    pub alpha: String,
    /// Whether a texture image was found and bound.
    pub textured: bool,
}

impl MeshNote {
    /// Whether the stage means the file produced no geometry or a doubtful one.
    #[must_use]
    pub fn is_issue(&self) -> bool {
        self.is_issue_stage() || !self.warnings.is_empty()
    }

    pub(super) fn is_issue_stage(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "rejected" | "extent" | "read-failed" | "parse-failed"
        )
    }
}
