//! Profile, path and rail blocks of a spline profile.

use super::*;

pub(super) fn handle_profile(
    block: &OmsiBlock,
    sli: &mut SplineProfile,
) -> Result<(), SceneryParseError> {
    match block.keyword.as_str() {
        "profile" => {
            if let Some(val) = block.parameters.first() {
                let tex_idx = parse_usize(val, "profile texture_index", block.line_number)?;
                sli.graphical_profiles.push(SplineGraphicalProfile {
                    texture_index: tex_idx,
                    points: Vec::new(),
                });
            }
        }
        "profilepnt" if block.parameters.len() >= 4 => {
            let x = parse_f32(&block.parameters[0], "profilepnt x", block.line_number)?;
            let z = parse_f32(&block.parameters[1], "profilepnt z", block.line_number)?;
            let u = parse_f32(&block.parameters[2], "profilepnt u", block.line_number)?;
            let vslope = parse_f32(
                &block.parameters[3],
                "profilepnt v_slope",
                block.line_number,
            )?;

            if let Some(prof) = sli.graphical_profiles.last_mut() {
                prof.points.push(SplineProfilePoint {
                    x,
                    z,
                    u,
                    v_slope: vslope,
                });
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_path(
    block: &OmsiBlock,
    sli: &mut SplineProfile,
) -> Result<(), SceneryParseError> {
    if block.parameters.len() >= 5 {
        let ptype = parse_u32(&block.parameters[0], "path type", block.line_number)?;
        let ox = parse_f32(&block.parameters[1], "path offset_x", block.line_number)?;
        let oz = parse_f32(&block.parameters[2], "path offset_z", block.line_number)?;
        let w = parse_f32(&block.parameters[3], "path width", block.line_number)?;
        let dir = parse_u32(&block.parameters[4], "path direction", block.line_number)?;
        let extra = block.parameters.get(5).and_then(|s| s.parse::<u32>().ok());

        sli.paths.push(SplinePath {
            path_type: ptype,
            offset_x: ox,
            offset_z: oz,
            width: w,
            direction: dir,
            extra,
        });
    }
    Ok(())
}

pub(super) fn handle_rail_enh(
    block: &OmsiBlock,
    sli: &mut SplineProfile,
) -> Result<(), SceneryParseError> {
    if block.parameters.len() >= 8 {
        let rlen = parse_f32(&block.parameters[0], "rail_enh length", block.line_number)?;
        let joints = parse_u32(&block.parameters[1], "rail_enh joints", block.line_number)?;
        let wl = parse_f32(
            &block.parameters[2],
            "rail_enh wavelength",
            block.line_number,
        )?;
        let amp = parse_f32(
            &block.parameters[3],
            "rail_enh amplitude",
            block.line_number,
        )?;
        let pwr = parse_f32(&block.parameters[4], "rail_enh power", block.line_number)?;
        let wlz = parse_f32(
            &block.parameters[5],
            "rail_enh wavelength_z",
            block.line_number,
        )?;
        let ampz = parse_f32(
            &block.parameters[6],
            "rail_enh amplitude_z",
            block.line_number,
        )?;
        let pwrz = parse_f32(&block.parameters[7], "rail_enh power_z", block.line_number)?;

        sli.rail_enh = Some(SplineRailEnh {
            rail_length: rlen,
            joints,
            wavelength: wl,
            amplitude: amp,
            power: pwr,
            wavelength_z: wlz,
            amplitude_z: ampz,
            power_z: pwrz,
        });
    }
    Ok(())
}
