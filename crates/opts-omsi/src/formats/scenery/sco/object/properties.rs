//! Render and physics property blocks of a scenery object (`[render]`, `[physics]` keywords).

use super::*;

pub(super) fn handle_render_property(
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

pub(super) fn handle_physics_property(
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
