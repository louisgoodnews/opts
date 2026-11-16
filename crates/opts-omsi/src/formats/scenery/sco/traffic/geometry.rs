//! Path, map light and spline helper records of a `.sco` file (the geometry keywords).

use super::*;

pub(super) fn parse_sco_path(
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

pub(super) fn parse_sco_maplight(
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

pub(super) fn parse_sco_splinehelper(
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
