//! Object-wide flags, names, groups, trees and the physical shape.

use super::*;

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(
    obj: &mut SceneryObject,
    st: &mut ParseState,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
        "groups" => {
            if let Some(first) = block.parameters.first() {
                let count = parse_usize(first, "groups count", block.line_number)?;
                obj.groups = block
                    .parameters
                    .iter()
                    .skip(1)
                    .take(count)
                    .cloned()
                    .collect();
            }
        }
        "new_attachment" => {
            obj.attachments.push(parse_attachment(&block.parameters));
        }
        "friendlyname" => {
            if let Some(name) = block.parameters.first() {
                obj.friendly_name = Some(name.clone());
            }
        }
        "complexity" => {
            if let Some(val) = block.parameters.first() {
                obj.complexity = Some(parse_u32(val, "complexity", block.line_number)?);
            }
        }
        "tree" => {
            if block.parameters.len() >= 5 {
                obj.tree = Some(ScoTree {
                    texture: block.parameters[0].clone(),
                    min_height: parse_f32(
                        &block.parameters[1],
                        "tree min height",
                        block.line_number,
                    )?,
                    max_height: parse_f32(
                        &block.parameters[2],
                        "tree max height",
                        block.line_number,
                    )?,
                    min_width_ratio: parse_f32(
                        &block.parameters[3],
                        "tree min width ratio",
                        block.line_number,
                    )?,
                    max_width_ratio: parse_f32(
                        &block.parameters[4],
                        "tree max width ratio",
                        block.line_number,
                    )?,
                });
            }
        }
        // Per mesh after a `[mesh]`, object-wide before the first one (installed corpus:
        // 2,356 of 3,371 files put it before their first mesh, 1,015 after a mesh, 941
        // of those `[tree]` helpers; `Eisplatte.sco` hides the second of two meshes).
        "onlyeditor" => match obj.meshes.last_mut() {
            Some(mesh) => mesh.only_editor = true,
            None => obj.only_editor = true,
        },
        "carpark_p" => {
            obj.carpark_p = true;
        }
        "model" => {
            if let Some(path) = block.parameters.first() {
                obj.model_cfg = Some(path.clone());
            }
        }
        "joinable" => {
            obj.joinable = true;
        }
        "helparrow" => {
            obj.is_help_arrow = true;
        }
        "fixed" => {
            obj.fixed = true;
        }
        "absheight" => {
            obj.abs_height = true;
        }
        "surface" => {
            obj.surface = true;
        }
        "nocollision" => {
            obj.no_collision = true;
        }
        "noshadow" => {
            if let Some(mesh) = obj.meshes.last_mut() {
                mesh.cast_shadow = Some(false);
            } else {
                obj.no_shadow = true;
            }
        }
        "shadow" => {
            if let Some(mesh) = obj.meshes.last_mut() {
                mesh.cast_shadow = Some(true);
            }
        }
        "nomaplighting" => {
            obj.no_map_lighting = true;
        }
        "rendertype" => {
            if let Some(val) = block.parameters.first() {
                obj.render_type = Some(val.clone());
            }
        }
        "nightmapmode" => {
            if let Some(val) = block.parameters.first() {
                obj.night_map_mode = Some(parse_u32(val, "NightMapMode", block.line_number)?);
            }
        }
        "lightmapmapping" => {
            obj.light_map_mapping = true;
        }
        "detail_factor" => {
            if let Some(val) = block.parameters.first() {
                obj.detail_factor = Some(parse_f32(val, "detail_factor", block.line_number)?);
            }
        }
        "nodistancecheck" => {
            obj.no_distance_check = true;
        }
        "trafficlight" => {
            obj.is_traffic_light = true;
        }
        "lod" => {
            if let Some(val) = block.parameters.first() {
                let threshold = parse_f32(val, "LOD", block.line_number)?;
                st.current_lod = Some(threshold);
            }
        }
        "crossing_heightdeformation" => {
            obj.crossing_deformation = block
                .parameters
                .first()
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty());
        }
        "collision_mesh" | "collisionsmesh" | "collisionmesh" | "collisioionsmesh" => {
            if let Some(file) = block.parameters.first() {
                obj.collision_meshes.push(file.clone());
            }
        }
        "boundingbox" => {
            if block.parameters.len() >= 6 {
                let hx = parse_f32(
                    &block.parameters[0],
                    "boundingbox half_x",
                    block.line_number,
                )?;
                let hy = parse_f32(
                    &block.parameters[1],
                    "boundingbox half_y",
                    block.line_number,
                )?;
                let hz = parse_f32(
                    &block.parameters[2],
                    "boundingbox half_z",
                    block.line_number,
                )?;
                let cx = parse_f32(&block.parameters[3], "boundingbox cx", block.line_number)?;
                let cy = parse_f32(&block.parameters[4], "boundingbox cy", block.line_number)?;
                let cz = parse_f32(&block.parameters[5], "boundingbox cz", block.line_number)?;
                obj.bounding_box = Some(ScoBoundingBox {
                    half_extents: Vec3::new(hx, hy, hz),
                    center: Vec3::new(cx, cy, cz),
                });
            }
        }
        "cog" => {
            if block.parameters.len() >= 3 {
                let x = parse_f32(&block.parameters[0], "cog x", block.line_number)?;
                let y = parse_f32(&block.parameters[1], "cog y", block.line_number)?;
                let z = parse_f32(&block.parameters[2], "cog z", block.line_number)?;
                obj.cog = Some(Vec3::new(x, y, z));
            }
        }
        "crashmode_pole" => {
            if block.parameters.len() >= 2 {
                let s = parse_f32(
                    &block.parameters[0],
                    "crashmode_pole stiffness",
                    block.line_number,
                )?;
                let f = parse_f32(
                    &block.parameters[1],
                    "crashmode_pole factor",
                    block.line_number,
                )?;
                obj.crashmode_pole = Some((s, f));
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
