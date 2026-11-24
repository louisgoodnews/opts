//! Block parser for `.bus` vehicle manifest files.

mod cameras;
mod chassis;
mod metadata;

use super::*;
use cameras::handle_cameras;
use chassis::{handle_chassis, handle_couplings};
use metadata::{handle_files, handle_metadata};

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
