//! Traffic light groups and programs, paths, map lights and spline helpers.

use super::*;

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(
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
                // 20,598 of 20,625 uses sit right after the phases of a light program.
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
        "path" | "path_2" => {
            if block.parameters.len() >= 11 {
                let x = parse_f32(&block.parameters[0], "path x", block.line_number)?;
                let y = parse_f32(&block.parameters[1], "path y", block.line_number)?;
                let z = parse_f32(&block.parameters[2], "path z", block.line_number)?;
                let rot = parse_f32(&block.parameters[3], "path rot", block.line_number)?;
                let rad = parse_f32(&block.parameters[4], "path radius", block.line_number)?;
                let len = parse_f32(&block.parameters[5], "path length", block.line_number)?;
                let s0 = parse_f32(&block.parameters[6], "path slope0", block.line_number)?;
                let s1 = parse_f32(&block.parameters[7], "path slope1", block.line_number)?;
                let ptype = parse_u32(&block.parameters[8], "path type", block.line_number)?;
                let width = parse_f32(&block.parameters[9], "path width", block.line_number)?;
                let dir = parse_u32(&block.parameters[10], "path dir", block.line_number)?;
                let extra = block.parameters.get(11).and_then(|s| s.parse::<u32>().ok());

                obj.paths.push(ScoPath {
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
                });
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
            if block.parameters.len() >= 7 {
                let x = parse_f32(&block.parameters[0], "maplight x", block.line_number)?;
                let y = parse_f32(&block.parameters[1], "maplight y", block.line_number)?;
                let z = parse_f32(&block.parameters[2], "maplight z", block.line_number)?;
                let r = parse_f32(&block.parameters[3], "maplight r", block.line_number)?;
                let g = parse_f32(&block.parameters[4], "maplight g", block.line_number)?;
                let b = parse_f32(&block.parameters[5], "maplight b", block.line_number)?;
                let rad = parse_f32(&block.parameters[6], "maplight radius", block.line_number)?;
                obj.map_lights.push(ScoMapLight {
                    position: Vec3::new(x, y, z),
                    color: Vec3::new(r, g, b),
                    radius: rad,
                });
            }
        }
        "splinehelper" => {
            if block.parameters.len() >= 5 {
                let file = block.parameters[0].clone();
                let x = parse_f32(&block.parameters[1], "splinehelper x", block.line_number)?;
                // Line order is x, height, y (verified on 2,274 helpers of X10 Berlin and
                // 509 of Bad Hügelsdorf: arms then meet the spline ends in plan and height).
                let z = parse_f32(&block.parameters[2], "splinehelper z", block.line_number)?;
                let y = parse_f32(&block.parameters[3], "splinehelper y", block.line_number)?;
                let rot = parse_f32(&block.parameters[4], "splinehelper rot", block.line_number)?;
                let mut extra = Vec::new();
                for p in &block.parameters[5..] {
                    if let Ok(v) = parse_float(p) {
                        extra.push(v);
                    }
                }
                obj.spline_helpers.push(ScoSplineHelper {
                    profile_file: file,
                    position: Vec3::new(x, y, z),
                    rotation_deg: rot,
                    extra,
                });
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
