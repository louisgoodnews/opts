//! Traffic light groups and programs, paths, map lights and spline helpers.

use super::*;

mod geometry;
use geometry::{parse_sco_maplight, parse_sco_path, parse_sco_splinehelper};

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
