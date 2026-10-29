//! Object-wide flags, names, groups, trees and the physical shape.

use super::*;

fn apply_flags(obj: &mut SceneryObject, keyword: &str) -> bool {
    match keyword {
        "carpark_p" => obj.carpark_p = true,
        "busstop" => obj.bus_stop = true,
        "joinable" => obj.joinable = true,
        "helparrow" => obj.is_help_arrow = true,
        "fixed" => obj.fixed = true,
        "absheight" => obj.abs_height = true,
        "surface" => obj.surface = true,
        "nocollision" => obj.no_collision = true,
        "nomaplighting" => obj.no_map_lighting = true,
        "lightmapmapping" => obj.light_map_mapping = true,
        "nodistancecheck" => obj.no_distance_check = true,
        "trafficlight" => obj.is_traffic_light = true,
        _ => return false,
    }
    true
}

fn parse_tree(params: &[String], line_number: usize) -> Result<Option<ScoTree>, SceneryParseError> {
    if params.len() < 5 {
        return Ok(None);
    }
    Ok(Some(ScoTree {
        texture: params[0].clone(),
        min_height: parse_f32(&params[1], "tree min height", line_number)?,
        max_height: parse_f32(&params[2], "tree max height", line_number)?,
        min_width_ratio: parse_f32(&params[3], "tree min width ratio", line_number)?,
        max_width_ratio: parse_f32(&params[4], "tree max width ratio", line_number)?,
    }))
}

fn parse_boundingbox(
    params: &[String],
    line_number: usize,
) -> Result<Option<ScoBoundingBox>, SceneryParseError> {
    if params.len() < 6 {
        return Ok(None);
    }
    let hx = parse_f32(&params[0], "boundingbox half_x", line_number)?;
    let hy = parse_f32(&params[1], "boundingbox half_y", line_number)?;
    let hz = parse_f32(&params[2], "boundingbox half_z", line_number)?;
    let cx = parse_f32(&params[3], "boundingbox cx", line_number)?;
    let cy = parse_f32(&params[4], "boundingbox cy", line_number)?;
    let cz = parse_f32(&params[5], "boundingbox cz", line_number)?;
    Ok(Some(ScoBoundingBox {
        half_extents: Vec3::new(hx, hy, hz),
        center: Vec3::new(cx, cy, cz),
    }))
}

fn parse_cog(params: &[String], line_number: usize) -> Result<Option<Vec3>, SceneryParseError> {
    if params.len() < 3 {
        return Ok(None);
    }
    let x = parse_f32(&params[0], "cog x", line_number)?;
    let y = parse_f32(&params[1], "cog y", line_number)?;
    let z = parse_f32(&params[2], "cog z", line_number)?;
    Ok(Some(Vec3::new(x, y, z)))
}

fn handle_render_property(
    obj: &mut SceneryObject,
    st: &mut ParseState,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
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
        "detail_factor" => {
            if let Some(val) = block.parameters.first() {
                obj.detail_factor = Some(parse_f32(val, "detail_factor", block.line_number)?);
            }
        }
        "lod" => {
            if let Some(val) = block.parameters.first() {
                let threshold = parse_f32(val, "LOD", block.line_number)?;
                st.current_lod = Some(threshold);
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn handle_physics_property(
    obj: &mut SceneryObject,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
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
            if let Some(v) = parse_boundingbox(&block.parameters, block.line_number)? {
                obj.bounding_box = Some(v);
            }
        }
        "cog" => {
            if let Some(v) = parse_cog(&block.parameters, block.line_number)? {
                obj.cog = Some(v);
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

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(
    obj: &mut SceneryObject,
    st: &mut ParseState,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    if apply_flags(obj, block.keyword.as_str()) {
        return Ok(true);
    }
    if handle_render_property(obj, st, block)? || handle_physics_property(obj, block)? {
        return Ok(true);
    }
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
            if let Some(v) = parse_tree(&block.parameters, block.line_number)? {
                obj.tree = Some(v);
            }
        }
        "onlyeditor" => match obj.meshes.last_mut() {
            Some(mesh) => mesh.only_editor = true,
            None => obj.only_editor = true,
        },
        "model" => {
            if let Some(path) = block.parameters.first() {
                obj.model_cfg = Some(path.clone());
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
