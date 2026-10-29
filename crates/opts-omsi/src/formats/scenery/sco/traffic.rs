//! Traffic light groups and programs, paths, map lights and spline helpers.

use super::*;

fn handle_traffic_light(
    obj: &mut SceneryObject,
    st: &mut ParseState,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
        "traffic_lights_group" => {
            if let Some(val) = block.parameters.first() {
                let cycle = parse_f32(val, "traffic_lights_group cycle", block.line_number)?;
                obj.traffic_light_groups.push(ScoTrafficLightGroup {
                    cycle_time_sec: cycle,
                    programs: Vec::new(),
                });
            }
        }
        "traffic_light" => {
            if let Some(name) = block.parameters.first() {
                let prog = ScoTrafficLightProgram {
                    name: name.clone(),
                    phases: Vec::new(),
                    approach_distance_m: None,
                };
                if let Some(grp) = obj.traffic_light_groups.last_mut() {
                    grp.programs.push(prog);
                } else {
                    // If program declared outside group, create implicit default group
                    obj.traffic_light_groups.push(ScoTrafficLightGroup {
                        cycle_time_sec: 0.0,
                        programs: vec![prog],
                    });
                }
            }
        }
        "phase" => {
            if block.parameters.len() >= 2 {
                let state = parse_u32(&block.parameters[0], "phase state", block.line_number)?;
                let dur = parse_f32(&block.parameters[1], "phase duration", block.line_number)?;
                let phase = ScoPhase {
                    state,
                    duration_sec: dur,
                };
                if let Some(grp) = obj.traffic_light_groups.last_mut() {
                    if let Some(prog) = grp.programs.last_mut() {
                        prog.phases.push(phase);
                    }
                }
            }
        }
        "use_traffic_light" => {
            if let Some(val) = block.parameters.first() {
                st.active_traffic_light =
                    Some(parse_usize(val, "use_traffic_light", block.line_number)?);
            }
        }
        "approachdist" => {
            if let Some(val) = block.parameters.first() {
                let dist = parse_f32(val, "approachdist", block.line_number)?;
                let program = if st.seen_path {
                    None
                } else {
                    obj.traffic_light_groups
                        .last_mut()
                        .and_then(|g| g.programs.last_mut())
                };
                match program {
                    Some(p) => p.approach_distance_m = Some(dist),
                    None => st.active_approach_dist = Some(dist),
                }
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn parse_sco_path(
    params: &[String],
    line_number: usize,
    st: &mut ParseState,
) -> Result<Option<ScoPath>, SceneryParseError> {
    if params.len() < 11 {
        return Ok(None);
    }
    let x = parse_f32(&params[0], "path x", line_number)?;
    let y = parse_f32(&params[1], "path y", line_number)?;
    let z = parse_f32(&params[2], "path z", line_number)?;
    let rot = parse_f32(&params[3], "path rot", line_number)?;
    let rad = parse_f32(&params[4], "path radius", line_number)?;
    let len = parse_f32(&params[5], "path length", line_number)?;
    let s0 = parse_f32(&params[6], "path slope0", line_number)?;
    let s1 = parse_f32(&params[7], "path slope1", line_number)?;
    let ptype = parse_u32(&params[8], "path type", line_number)?;
    let width = parse_f32(&params[9], "path width", line_number)?;
    let dir = parse_u32(&params[10], "path dir", line_number)?;
    let extra = params.get(11).and_then(|s| s.parse::<u32>().ok());

    Ok(Some(ScoPath {
        start: Vec3::new(x, y, z),
        heading_deg: rot,
        radius_m: rad,
        length_m: len,
        slope_start: s0,
        slope_end: s1,
        path_type: ptype,
        width_m: width,
        direction: dir,
        extra,
        traffic_light_index: st.active_traffic_light.take(),
        approach_distance_m: st.active_approach_dist,
        switch_position: st.switch_position,
    }))
}

fn parse_sco_maplight(
    params: &[String],
    line_number: usize,
) -> Result<Option<ScoMapLight>, SceneryParseError> {
    if params.len() < 7 {
        return Ok(None);
    }
    let x = parse_f32(&params[0], "maplight x", line_number)?;
    let y = parse_f32(&params[1], "maplight y", line_number)?;
    let z = parse_f32(&params[2], "maplight z", line_number)?;
    let r = parse_f32(&params[3], "maplight r", line_number)?;
    let g = parse_f32(&params[4], "maplight g", line_number)?;
    let b = parse_f32(&params[5], "maplight b", line_number)?;
    let rad = parse_f32(&params[6], "maplight radius", line_number)?;
    Ok(Some(ScoMapLight {
        position: Vec3::new(x, y, z),
        color: Vec3::new(r, g, b),
        radius: rad,
    }))
}

fn parse_sco_splinehelper(
    params: &[String],
    line_number: usize,
) -> Result<Option<ScoSplineHelper>, SceneryParseError> {
    if params.len() < 5 {
        return Ok(None);
    }
    let file = params[0].clone();
    let x = parse_f32(&params[1], "splinehelper x", line_number)?;
    let z = parse_f32(&params[2], "splinehelper z", line_number)?;
    let y = parse_f32(&params[3], "splinehelper y", line_number)?;
    let rot = parse_f32(&params[4], "splinehelper rot", line_number)?;
    let mut extra = Vec::new();
    for p in &params[5..] {
        if let Ok(v) = parse_float(p) {
            extra.push(v);
        }
    }
    Ok(Some(ScoSplineHelper {
        profile_file: file,
        position: Vec3::new(x, y, z),
        rotation_deg: rot,
        extra,
    }))
}

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(
    obj: &mut SceneryObject,
    st: &mut ParseState,
    block: &OmsiBlock,
) -> Result<bool, SceneryParseError> {
    if handle_traffic_light(obj, st, block)? {
        return Ok(true);
    }
    match block.keyword.as_str() {
        "path" | "path_2" => {
            if let Some(path) = parse_sco_path(&block.parameters, block.line_number, st)? {
                obj.paths.push(path);
                st.seen_path = true;
            }
        }
        "switch" => {
            if let Some(n) = block.parameters.first().and_then(|s| s.parse::<u32>().ok()) {
                obj.switch_positions = n;
            }
        }
        "switchdir" => {
            st.switch_position = block.parameters.first().and_then(|s| s.parse::<u32>().ok());
        }
        "maplight" => {
            if let Some(light) = parse_sco_maplight(&block.parameters, block.line_number)? {
                obj.map_lights.push(light);
            }
        }
        "splinehelper" => {
            if let Some(helper) = parse_sco_splinehelper(&block.parameters, block.line_number)? {
                obj.spline_helpers.push(helper);
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
