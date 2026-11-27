//! Shared fixture of the kinematics suite: the synthetic transit bus manifest.

use opts_core::formats::vehicles::bus::{BusAxle, BusManifest, RegistrationMode};

/// Helper constructing a synthetic standard transit bus manifest.
pub(super) fn make_test_manifest(
    mass_tonnes: Option<f32>,
    wheel_diameter: Option<f32>,
) -> BusManifest {
    BusManifest {
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
        mass_tonnes,
        bounding_box: None,
        vehicle_type: None,
        bogie_half_span: None,
        couple_back_reversed: false,
        couple_front: None,
        couple_front_reversed: false,
        axles: vec![
            BusAxle {
                longitudinal_m: 3.25,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: wheel_diameter,
                spring_rate: Some(280.0),
                max_force: Some(120.0),
                damping: Some(18.0),
                is_driven: false,
            },
            BusAxle {
                longitudinal_m: -2.60,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: wheel_diameter,
                spring_rate: Some(340.0),
                max_force: Some(150.0),
                damping: Some(22.0),
                is_driven: true,
            },
        ],
        coupling_front_y: None,
        coupling_back_y: None,
        coupling_front: None,
        coupling_back: None,
        couple_back: None,
        coupling_front_limits: None,
        driver_cameras: Vec::new(),
        pax_cameras: Vec::new(),
        mirror_cameras: Vec::new(),
        std_camera_index: None,
        outside_camera_center: None,
        extras: Default::default(),
    }
}
