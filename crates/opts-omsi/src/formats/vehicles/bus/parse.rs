//! Block parser for `.bus` vehicle manifest files.

use super::*;
use crate::formats::text::OmsiBlock;
use blocks::{extract_counted_list, parse_axle_block, parse_camera_block, parse_mirror_block};

fn handle_metadata(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "friendlyname" => {
            if block.parameters.len() >= 3 {
                m.friendly_name = Some(FriendlyName {
                    manufacturer: block.parameters[0].clone(),
                    model: block.parameters[1].clone(),
                    livery: block.parameters[2].clone(),
                });
            }
        }
        "description" => {
            for line in &block.parameters {
                if line == "[end]" {
                    break;
                }
                m.description.push(line.clone());
            }
        }
        "number" => {
            if let Some(first) = block.parameters.first() {
                m.number_file = Some(first.clone());
            }
        }
        "registration_list" => {
            if let Some(first) = block.parameters.first() {
                m.registration_mode = RegistrationMode::List(first.clone());
            }
        }
        "registration_automatic" => {
            if let Some(first) = block.parameters.first() {
                m.registration_mode = RegistrationMode::Automatic(first.clone());
            }
        }
        "registration_free" => {
            m.registration_mode = RegistrationMode::Free;
        }
        _ => {}
    }
}

fn handle_files(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "sound" => {
            if let Some(first) = block.parameters.first() {
                m.sound_cfg = Some(first.clone());
            }
        }
        "sound_ai" => {
            if let Some(first) = block.parameters.first() {
                m.sound_ai_cfg = Some(first.clone());
            }
        }
        "model" => {
            if let Some(first) = block.parameters.first() {
                m.model_cfg = Some(first.clone());
            }
        }
        "paths" => {
            if let Some(first) = block.parameters.first() {
                m.paths_cfg = Some(first.clone());
            }
        }
        "passengercabin" => {
            if let Some(first) = block.parameters.first() {
                m.passengercabin_cfg = Some(first.clone());
            }
        }
        "varnamelist" => extract_counted_list(&block.parameters, &mut m.varnamelists),
        "stringvarnamelist" => extract_counted_list(&block.parameters, &mut m.stringvarnamelists),
        "script" => extract_counted_list(&block.parameters, &mut m.scripts),
        "constfile" => extract_counted_list(&block.parameters, &mut m.constfiles),
        "scriptshare" => m.script_share = true,
        _ => {}
    }
}

fn handle_mass(block: &OmsiBlock, m: &mut BusManifest, legacy_inertia: &mut Option<[f32; 3]>) {
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

fn handle_chassis(block: &OmsiBlock, m: &mut BusManifest, legacy_inertia: &mut Option<[f32; 3]>) {
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

fn handle_couplings(block: &OmsiBlock, m: &mut BusManifest) {
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

fn handle_cameras(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "add_camera_driver" if block.parameters.len() >= 7 => {
            if let Some(cam) = parse_camera_block(&block.parameters, true) {
                m.driver_cameras.push(cam);
            }
        }
        "add_camera_pax" if block.parameters.len() >= 7 => {
            if let Some(cam) = parse_camera_block(&block.parameters, false) {
                m.pax_cameras.push(cam);
            }
        }
        "add_camera_reflexion" | "add_camera_reflexion_2" => {
            if let Some(cam) = parse_mirror_block(&block.parameters) {
                m.mirror_cameras.push(cam);
            }
        }
        "set_camera_std" => {
            if let Some(first) = block.parameters.first() {
                m.std_camera_index = first.trim().parse::<usize>().ok();
            }
        }
        "set_camera_outside_center" if block.parameters.len() >= 3 => {
            let x = parse_float(&block.parameters[0]).unwrap_or(0.0);
            let y = parse_float(&block.parameters[1]).unwrap_or(0.0);
            let z = parse_float(&block.parameters[2]).unwrap_or(0.0);
            m.outside_camera_center = Some(glam::Vec3::new(x, y, z));
        }
        "view_schedule" => {
            if let Some(cam) = m.driver_cameras.last_mut() {
                cam.is_schedule_view = true;
            }
        }
        "view_ticketselling" => {
            if let Some(cam) = m.driver_cameras.last_mut() {
                cam.is_ticket_view = true;
            }
        }
        _ => {}
    }
}

impl BusManifest {
    /// Parses a `.bus` vehicle manifest from text.
    ///
    /// # Errors
    /// Returns [`BusParseError`] if parameters cannot be parsed.
    pub fn parse(text: &str) -> Result<Self, BusParseError> {
        let blocks = scan_blocks(text);
        let mut m = Self {
            friendly_name: None,
            description: Vec::new(),
            number_file: None,
            registration_mode: RegistrationMode::None,
            sound_cfg: None,
            sound_ai_cfg: None,
            model_cfg: None,
            paths_cfg: None,
            passengercabin_cfg: None,
            varnamelists: Vec::new(),
            stringvarnamelists: Vec::new(),
            scripts: Vec::new(),
            constfiles: Vec::new(),
            script_share: false,
            vehicle_type: None,
            bogie_half_span: None,
            mass_tonnes: None,
            bounding_box: None,
            axles: Vec::new(),
            coupling_front_y: None,
            coupling_back_y: None,
            coupling_front: None,
            coupling_back: None,
            couple_back: None,
            couple_back_reversed: false,
            coupling_front_limits: None,
            driver_cameras: Vec::new(),
            pax_cameras: Vec::new(),
            mirror_cameras: Vec::new(),
            std_camera_index: None,
            outside_camera_center: None,
            extras: VehicleExtras::default(),
        };
        let mut legacy_inertia: Option<[f32; 3]> = None;

        for block in blocks {
            if m.extras.apply_block(&block) {
                continue;
            }
            handle_metadata(&block, &mut m);
            handle_files(&block, &mut m);
            handle_chassis(&block, &mut m, &mut legacy_inertia);
            handle_couplings(&block, &mut m);
            handle_cameras(&block, &mut m);
        }

        if m.extras.inertia_t_m2.is_none() {
            m.extras.inertia_t_m2 = legacy_inertia;
        }

        Ok(m)
    }

    /// Parses a `.bus` vehicle manifest from raw bytes.
    ///
    /// # Errors
    /// Returns [`BusParseError`] on syntax error.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, BusParseError> {
        let text = decode_bytes(bytes);
        Self::parse(&text)
    }
}
