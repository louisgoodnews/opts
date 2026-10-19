//! The per-load trace scratch.

use super::*;

/// What the scene trace learns while one mesh file loads (costs nothing when the trace is off:
/// `Lap` is inactive and no string is built).
pub(super) struct TraceScratch {
    pub(super) lap: crate::scene_trace::Lap,
    pub(super) stage: &'static str,
    pub(super) read_ms: f32,
    pub(super) parse_ms: f32,
    pub(super) decode_ms: f32,
    pub(super) build_ms: f32,
    pub(super) hidden: usize,
    pub(super) materials: Vec<crate::scene_trace::MaterialNote>,
    pub(super) warnings: Vec<String>,
}

impl TraceScratch {
    pub(super) fn new() -> Self {
        Self {
            lap: crate::scene_trace::Lap::start(),
            stage: "",
            read_ms: 0.0,
            parse_ms: 0.0,
            decode_ms: 0.0,
            build_ms: 0.0,
            hidden: 0,
            materials: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub(super) fn warn(&mut self, text: impl FnOnce() -> String) {
        if self.lap.active() {
            self.warnings.push(text());
        }
    }

    /// Hands the finished note to the trace.
    pub(super) fn finish(self, meshes: &Assets<Mesh>, path: &Path, parts: &[ModelPart]) {
        use crate::scene_trace as st;
        let had_note = st::has_note(path);
        if self.stage == "parts-cache" && had_note {
            return;
        }
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        let mut any = false;
        for p in parts {
            let Some(pos) = meshes
                .get(&p.mesh)
                .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
                .and_then(|a| a.as_float3())
            else {
                continue;
            };
            for v in pos {
                any = true;
                for i in 0..3 {
                    lo[i] = lo[i].min(v[i]);
                    hi[i] = hi[i].max(v[i]);
                }
            }
        }
        let mut warnings = self.warnings;
        if self.hidden > 0 && parts.is_empty() {
            warnings.clear();
        }
        let stage = if self.stage == "parts-cache" {
            "cached"
        } else if parts.is_empty() && self.hidden > 0 && self.stage == "plain" {
            "hidden"
        } else {
            self.stage
        };
        st::note_mesh(
            path,
            st::MeshNote {
                stage: stage.into(),
                read_ms: self.read_ms,
                parse_ms: self.parse_ms,
                decode_ms: self.decode_ms,
                build_ms: self.build_ms,
                vertices: parts.iter().map(|p| p.vertices).sum(),
                parts: parts.len(),
                materials: self.materials,
                warnings,
                bbox: any.then_some(st::TraceBounds { min: lo, max: hi }),
                ..st::MeshNote::default()
            },
        );
    }
}
