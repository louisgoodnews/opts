//! Block parser for OMSI spline profile (`.sli`) files.

use super::*;

impl SplineProfile {
    /// Parses a spline profile from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, SceneryParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a spline profile from decoded text.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, SceneryParseError> {
        let blocks = scan_blocks(text);
        let mut sli = SplineProfile::default();

        for block in &blocks {
            match block.keyword.as_str() {
                "length" => {
                    if let Some(val) = block.parameters.first() {
                        sli.length = Some(parse_f32(val, "length", block.line_number)?);
                    }
                }
                "halfcantwidth" => {
                    if let Some(val) = block.parameters.first() {
                        sli.half_cant_width =
                            Some(parse_f32(val, "halfcantwidth", block.line_number)?);
                    }
                }
                "scaletexbylength" => {
                    sli.scale_tex_by_length = true;
                }
                "rendertype" => {
                    if let Some(val) = block.parameters.first() {
                        sli.render_type = Some(val.clone());
                    }
                }
                "onlyeditor" => {
                    sli.only_editor = true;
                }
                "heightprofile" | "heightproflile" | "heigthprofile" => {
                    if block.parameters.len() >= 4 {
                        let xs = parse_f32(
                            &block.parameters[0],
                            "heightprofile x_start",
                            block.line_number,
                        )?;
                        let xe = parse_f32(
                            &block.parameters[1],
                            "heightprofile x_end",
                            block.line_number,
                        )?;
                        let zs = parse_f32(
                            &block.parameters[2],
                            "heightprofile z_start",
                            block.line_number,
                        )?;
                        let ze = parse_f32(
                            &block.parameters[3],
                            "heightprofile z_end",
                            block.line_number,
                        )?;
                        sli.height_profiles.push(SplineHeightProfile {
                            x_start: xs,
                            x_end: xe,
                            z_start: zs,
                            z_end: ze,
                        });
                    }
                }
                "texture" => {
                    if let Some(file) = block.parameters.first() {
                        sli.textures.push(SplineTexture {
                            file: file.clone(),
                            material_index: None,
                            alpha_mode: None,
                        });
                    }
                }
                "matl" => {
                    if block.parameters.len() >= 2 {
                        let idx =
                            parse_usize(&block.parameters[1], "matl index", block.line_number)?;
                        if let Some(tex) = sli.textures.last_mut() {
                            tex.material_index = Some(idx);
                        }
                    }
                }
                "matl_alpha" => {
                    if let Some(val) = block.parameters.first() {
                        let mode = parse_u32(val, "matl_alpha", block.line_number)?;
                        if let Some(tex) = sli.textures.last_mut() {
                            tex.alpha_mode = Some(mode);
                        }
                    }
                }
                "profile" => {
                    if let Some(val) = block.parameters.first() {
                        let tex_idx = parse_usize(val, "profile texture_index", block.line_number)?;
                        sli.graphical_profiles.push(SplineGraphicalProfile {
                            texture_index: tex_idx,
                            points: Vec::new(),
                        });
                    }
                }
                "profilepnt" => {
                    if block.parameters.len() >= 4 {
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
                }
                "path" | "path_2" => {
                    if block.parameters.len() >= 5 {
                        let ptype =
                            parse_u32(&block.parameters[0], "path type", block.line_number)?;
                        let ox =
                            parse_f32(&block.parameters[1], "path offset_x", block.line_number)?;
                        let oz =
                            parse_f32(&block.parameters[2], "path offset_z", block.line_number)?;
                        let w = parse_f32(&block.parameters[3], "path width", block.line_number)?;
                        let dir =
                            parse_u32(&block.parameters[4], "path direction", block.line_number)?;
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
                }
                "rail_enh" if block.parameters.len() >= 8 => {
                    let rlen =
                        parse_f32(&block.parameters[0], "rail_enh length", block.line_number)?;
                    let joints =
                        parse_u32(&block.parameters[1], "rail_enh joints", block.line_number)?;
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
                    let pwrz =
                        parse_f32(&block.parameters[7], "rail_enh power_z", block.line_number)?;

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
                _ => {
                    // Unknown or comment block - ignore gracefully
                }
            }
        }

        Ok(sli)
    }
}

fn parse_f32(s: &str, field: &'static str, line: usize) -> Result<f32, SceneryParseError> {
    parse_float(s).map_err(|_| SceneryParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

fn parse_u32(s: &str, field: &'static str, line: usize) -> Result<u32, SceneryParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

fn parse_usize(s: &str, field: &'static str, line: usize) -> Result<usize, SceneryParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}
