//! Mass, chassis, axles and couplings of a `.bus` manifest.

use super::super::blocks::parse_axle_block;
use super::super::*;
use crate::formats::text::OmsiBlock;

pub(super) fn handle_mass(
    block: &OmsiBlock,
    m: &mut BusManifest,
    legacy_inertia: &mut Option<[f32; 3]>,
) {
    if let Some(first) = block.parameters.first() {
        if let Ok(mass) = parse_float(first) {
            m.mass_tonnes = Some(mass);
        }
    }
    let legacy: Vec<f32> = block
        .parameters
        .iter()
        .skip(1)
        .take(3)
        .map_while(|p| parse_float(p).ok())
        .collect();
    if let [x, y, z] = legacy[..] {
        if (x, y, z) != (0.0, 0.0, 0.0) {
            *legacy_inertia = Some([x, y, z]);
        }
    }
}

pub(super) fn handle_chassis(
    block: &OmsiBlock,
    m: &mut BusManifest,
    legacy_inertia: &mut Option<[f32; 3]>,
) {
    match block.keyword.as_str() {
        "boundingbox" => {
            let v: Vec<f32> = block
                .parameters
                .iter()
                .take(6)
                .filter_map(|p| parse_float(p).ok())
                .collect();
            if v.len() == 6 {
                m.bounding_box = Some((
                    glam::Vec3::new(v[0], v[1], v[2]),
                    glam::Vec3::new(v[3], v[4], v[5]),
                ));
            }
        }
        "type" => {
            m.vehicle_type = block
                .parameters
                .first()
                .and_then(|p| p.trim().parse::<u32>().ok());
        }
        "boogies" => {
            m.bogie_half_span = block
                .parameters
                .first()
                .and_then(|p| parse_float(p).ok())
                .filter(|y| *y > 0.0);
        }
        "mass" => handle_mass(block, m, legacy_inertia),
        "newachse" => {
            let mut axle = BusAxle {
                longitudinal_m: 0.0,
                max_width_m: None,
                min_width_m: None,
                wheel_diameter_m: None,
                spring_rate: None,
                max_force: None,
                damping: None,
                is_driven: false,
            };
            parse_axle_block(&block.parameters, &mut axle);
            m.axles.push(axle);
        }
        _ => {}
    }
}

pub(super) fn handle_couplings(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "coupling_front" | "coupling_back" => {
            let v: Vec<f32> = block
                .parameters
                .iter()
                .take(3)
                .filter_map(|p| parse_float(p).ok())
                .collect();
            let front = block.keyword == "coupling_front";
            if v.len() >= 3 {
                let point = glam::Vec3::new(v[0], v[1], v[2]);
                if front {
                    m.coupling_front = Some(point);
                    m.coupling_front_y = Some(point.y);
                } else {
                    m.coupling_back = Some(point);
                    m.coupling_back_y = Some(point.y);
                }
            } else if let [y] = v[..] {
                if front {
                    m.coupling_front_y = Some(y);
                } else {
                    m.coupling_back_y = Some(y);
                }
            }
        }
        "coupling_front_character" => {
            let v: Vec<f32> = block
                .parameters
                .iter()
                .take(4)
                .filter_map(|p| parse_float(p).ok())
                .collect();
            if let [a, b0, b1, k] = v[..] {
                m.coupling_front_limits = Some(CouplingLimits {
                    max_alpha_deg: a,
                    min_beta_deg: b0,
                    max_beta_deg: b1,
                    kind: k.max(0.0) as u32,
                });
            }
        }
        "couple_back" => {
            m.couple_back = block
                .parameters
                .first()
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty());
            m.couple_back_reversed = block
                .parameters
                .get(1)
                .is_some_and(|f| f.trim().eq_ignore_ascii_case("true"));
        }
        _ => {}
    }
}
