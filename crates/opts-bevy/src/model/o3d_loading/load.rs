//! Loading one O3D mesh file into model parts: the cached parse-and-build path.

use super::part_builder::SlotBuildCtx;
use super::validate::decode_and_validate_o3d;
use super::*;

impl ContentLoader<'_> {
    pub(super) fn o3d_parts_core(
        &mut self,
        args: O3dPartsArgs<'_>,
        sc: &mut TraceScratch,
    ) -> Vec<ModelPart> {
        let mut repaint: Vec<_> = self.cache.texture_replacements.iter().collect();
        repaint.sort();
        let key = parts_key(
            args.o3d_path,
            args.overrides,
            args.keep_skin,
            &repaint,
            args.slots,
            args.bend.map(|b| b.0),
        );
        if let Some(parts) = self.cache.parts.get(&key) {
            sc.stage = "parts-cache";
            if self.cache.protected.contains(args.o3d_path) {
                self.cache.stats.protected_meshes += 1;
            }
            return parts.clone();
        }

        let _t_miss = opts_logging::stream_trace::span("o3d.parts_miss");
        let Some(mut models) = self.read_and_parse_mesh(args.o3d_path, sc) else {
            return Vec::new();
        };

        if !decode_and_validate_o3d(&mut models, args.o3d_path, sc, self.cache) {
            self.cache.parts.insert(key, Vec::new());
            return Vec::new();
        }
        if let Some((_, base)) = args.bend {
            for m in &mut models {
                base.bend_mesh(m);
            }
        }
        for model in &models {
            self.cache.stats.o3d_files += 1;
            self.cache.stats.o3d_vertices += model.vertices.len();
            tracing::debug!(
                path = %args.o3d_path.display(),
                vertices = model.vertices.len(),
                triangles = model.indices.len(),
                materials = model.materials.len(),
                "loaded mesh"
            );
        }

        let ctx = SlotBuildCtx {
            o3d_path: args.o3d_path,
            tex_dirs: args.tex_dirs,
            overrides: args.overrides,
            keep_skin: args.keep_skin,
            slots: args.slots,
        };
        let mut parts = Vec::new();
        for model in &models {
            self.build_model_parts(model, &ctx, sc, &mut parts);
        }
        sc.build_ms = sc.lap.split();
        self.cache.parts.insert(key, parts.clone());
        parts
    }

    fn read_and_parse_mesh(&mut self, path: &Path, sc: &mut TraceScratch) -> Option<Vec<O3d>> {
        let t_parse = opts_logging::stream_trace::span("o3d.read_parse");
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                sc.stage = "read-failed";
                sc.warn(|| format!("read failed: {e}"));
                self.fail(format!("o3d read failed {}: {e}", path.display()));
                return None;
            }
        };
        sc.read_ms = sc.lap.split();
        let models: Vec<O3d> = match crate::mesh::parse_mesh_file(path, &bytes) {
            Ok(m) => m,
            Err(e) => {
                sc.stage = "parse-failed";
                sc.warn(|| format!("parse failed: {e}"));
                self.fail(format!("mesh parse failed {}: {e}", path.display()));
                return None;
            }
        };
        sc.parse_ms = sc.lap.split();
        sc.stage = "plain";
        drop(t_parse);
        Some(models)
    }
}
