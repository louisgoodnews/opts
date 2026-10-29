//! Meshes, their materials, lights, visibility, mouse events and animations.

use super::*;

fn handle_sco_material(
    mat: &mut ScoMaterial,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
        "matl_alpha" => {
            if let Some(val) = block.parameters.first() {
                mat.alpha_mode = Some(parse_u32(val, "matl_alpha", block.line_number)?);
            }
        }
        "matl_envmap" => {
            if block.parameters.len() >= 2 {
                let env_file = block.parameters[0].clone();
                let strength = parse_f32(
                    &block.parameters[1],
                    "matl_envmap strength",
                    block.line_number,
                )?;
                mat.env_map = Some((env_file, strength));
            }
        }
        "matl_envmap_mask" => {
            mat.env_map_mask = true;
        }
        "matl_lightmap" => {
            if let Some(val) = block.parameters.first() {
                mat.light_map = Some(val.clone());
                mat.light_map_variable = light_map_variable(&block.parameters);
            }
        }
        "matl_nightmap" => {
            if let Some(val) = block.parameters.first() {
                mat.night_map = Some(val.clone());
            }
        }
        "matl_transmap" => {
            mat.trans_map = true;
        }
        "matl_nozwrite" => {
            mat.no_z_write = true;
        }
        "matl_nozcheck" => {
            mat.no_z_check = true;
        }
        "matl_freetex" => {
            if let Some(val) = block.parameters.first() {
                mat.free_tex = Some(val.clone());
            }
        }
        "matl_change" => {
            if block.parameters.len() >= 3 {
                let tex = block.parameters[0].clone();
                let idx =
                    parse_usize(&block.parameters[1], "matl_change index", block.line_number)?;
                let var = block.parameters[2].clone();
                mat.matl_change = Some((tex, idx, var));
            }
        }
        "alphascale" => {
            if let Some(val) = block.parameters.first() {
                mat.alpha_scale = Some(val.clone());
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
    // Material keywords are parsed (and their numbers checked) even before a mesh or material
    // exists; the values then go nowhere.
    let mut scratch = ScoMaterial::new(String::new(), 0);
    let mat = obj
        .meshes
        .last_mut()
        .and_then(|m| m.materials.last_mut())
        .unwrap_or(&mut scratch);
    if handle_sco_material(mat, block)? {
        return Ok(true);
    }
    match block.keyword.as_str() {
        "mesh" => {
            if let Some(file) = block.parameters.first() {
                let mut mesh = ScoMesh::new(file.clone());
                mesh.lod_threshold = st.current_lod;
                obj.meshes.push(mesh);
            }
        }
        "matl" => {
            if block.parameters.len() >= 2 {
                let tex = &block.parameters[0];
                let mat_idx = parse_usize(&block.parameters[1], "matl index", block.line_number)?;
                let mat = ScoMaterial::new(tex.clone(), mat_idx);
                if let Some(mesh) = obj.meshes.last_mut() {
                    mesh.materials.push(mat);
                }
            }
        }
        "light_enh" | "light_enh_2" => add_light(obj, block, st.current_lod),
        "visible" => {
            if block.parameters.len() >= 2 {
                let var = block.parameters[0].clone();
                let val = parse_f32(&block.parameters[1], "visible value", block.line_number)?;
                if let Some(mesh) = obj.meshes.last_mut() {
                    mesh.visible = Some((var, val));
                }
            }
        }
        "mouseevent" => {
            if let Some(val) = block.parameters.first() {
                if let Some(mesh) = obj.meshes.last_mut() {
                    mesh.mouse_event = Some(val.clone());
                }
            }
        }
        "usetexttexture" => {
            if let Some(val) = block.parameters.first() {
                let idx = parse_usize(val, "useTextTexture index", block.line_number)?;
                if let Some(mesh) = obj.meshes.last_mut() {
                    mesh.text_texture_indices.push(idx);
                    if let Some(mat) = mesh.materials.last_mut() {
                        mat.text_texture = Some(idx);
                    }
                }
            }
        }
        "newanim" => match parse_newanim(block) {
            Ok(anim) => {
                if let Some(mesh) = obj.meshes.last_mut() {
                    mesh.animations.push(anim);
                }
            }
            Err(e) => obj.warnings.push(format!(
                "[newanim] skipped (line {}): {e}",
                block.line_number
            )),
        },
        _ => return Ok(false),
    }
    Ok(true)
}
