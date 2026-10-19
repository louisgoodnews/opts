//! Parser and data model for OMSI vehicle manifests (`.bus`).
//!
//! A `.bus` file is the central manifest for a drivable or AI vehicle in OMSI.
//! It declares identity metadata, references to model, sound, and path files,
//! the script system files, physical properties (mass, axles), and coupling links.
//!
//! See specification in `docs/src/formats/text/vehicles/bus.md`.

use super::error::BusParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

/// Human-readable vehicle identity for selection menus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendlyName {
    /// Manufacturer or brand (e.g. `"MAN"`).
    pub manufacturer: String,
    /// Vehicle model designation (e.g. `"NL202 - EN92"`).
    pub model: String,
    /// Default livery or paint scheme name (e.g. `"BVG Standard"`).
    pub livery: String,
}

/// Registration / license plate configuration mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationMode {
    /// License plate resolved from a dedicated mapping file.
    List(String),
    /// Automatic prefix-based generation.
    Automatic(String),
    /// Free player selection allowed.
    Free,
    /// No registration keyword configured.
    None,
}

/// Limits of a coupling (`[coupling_front_character]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CouplingLimits {
    /// Largest yaw angle `α` in degrees (corpus: 38–110, mostly 50–57).
    pub max_alpha_deg: f32,
    /// Smallest pitch `β` in degrees (negative = down).
    pub min_beta_deg: f32,
    /// Largest pitch `β` in degrees.
    pub max_beta_deg: f32,
    /// `0` = truck coupling (three degrees of freedom), `1` = bus joint (two).
    pub kind: u32,
}

/// Physical parameters of a vehicle axle.
#[derive(Debug, Clone, PartialEq)]
pub struct BusAxle {
    /// Longitudinal position along Y-axis in meters.
    pub longitudinal_m: f32,
    /// Maximum wheel track width in meters.
    pub max_width_m: Option<f32>,
    /// Minimum wheel track width in meters.
    pub min_width_m: Option<f32>,
    /// Wheel outer diameter in meters.
    pub wheel_diameter_m: Option<f32>,
    /// Suspension spring stiffness rate.
    pub spring_rate: Option<f32>,
    /// Maximum suspension travel force.
    pub max_force: Option<f32>,
    /// Shock absorber damping coefficient.
    pub damping: Option<f32>,
    /// Whether this axle receives engine powertrain torque.
    pub is_driven: bool,
}

/// Vehicle view camera preset (`[add_camera_driver]`, `[add_camera_pax]`).
#[derive(Debug, Clone, PartialEq)]
pub struct BusCamera {
    /// Lateral offset in meters (OMSI: positive = left).
    pub x: f32,
    /// Longitudinal offset in meters (positive = forward).
    pub y: f32,
    /// Vertical offset in meters (positive = up).
    pub z: f32,
    /// Eye distance from pivot point in meters (slightly negative for driver cameras).
    pub dist: f32,
    /// Horizontal field of view in degrees.
    pub fov_deg: f32,
    /// Horizontal viewing direction in degrees (-120 full left, 0 forward, +120 full right).
    pub yaw_deg: f32,
    /// Vertical viewing direction in degrees (negative = down).
    pub pitch_deg: f32,
    /// Whether this is a driver view camera.
    pub is_driver: bool,
    /// Whether marked as timetable schedule view (`[view_schedule]`).
    pub is_schedule_view: bool,
    /// Whether marked as ticket selling view (`[view_ticketselling]`).
    pub is_ticket_view: bool,
}

/// Mirror / monitor camera (`[add_camera_reflexion]`, `[add_camera_reflexion_2]`).
///
/// The Nth tag of the file (both keywords share one counter) renders into the texture
/// `reflexion<N>.bmp` of the vehicle, which the model's mirror pane shows. The frame is the
/// vehicle's, like the driver cameras (see `docs/src/formats/text/vehicles/bus.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct BusMirrorCamera {
    /// Lateral offset in meters (the files label negative values as the left side).
    pub x: f32,
    /// Longitudinal offset in meters (positive = forward).
    pub y: f32,
    /// Vertical offset in meters (positive = up).
    pub z: f32,
    /// Fourth number: `0` in all but a few files, meaning unknown.
    pub param4: f32,
    /// Field of view in degrees (the textures are square, so horizontal = vertical).
    pub fov_deg: f32,
    /// Viewing direction around the vertical axis in degrees (about 180 = backwards).
    pub yaw_deg: f32,
    /// Pitch in degrees (negative = down).
    pub pitch_deg: f32,
    /// Eighth number, only present on `[add_camera_reflexion_2]`; meaning unknown (0.0 to 6,
    /// larger on door and blind-spot cameras).
    pub param8: Option<f32>,
}

/// Keywords that shape how a vehicle drives or is driven by AI, beyond mass and axles.
///
/// Counts are files of the installation that declare the keyword (2,752 `.bus`/`.ovh` files).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleExtras {
    /// `[ai_veh_type]`: `0` passenger car, `1` cab, `2` bus, `3` truck (the legend in the vehicle
    /// files themselves; 80 files).
    pub ai_veh_type: Option<u32>,
    /// `[ai_brakeperformance]`: the numbers of the block in file order, the first being the
    /// brake multiplier (141 files; the others are unexplained).
    pub ai_brake_performance: Vec<f32>,
    /// `[rowdy_factor]`: `(low, high)`, a per-vehicle random draw between the two at creation
    /// (539 files).
    pub rowdy_factor: Option<(f32, f32)>,
    /// `[ai_deltaheight]`: vertical offset when driven by AI, metres (1,485 files).
    pub ai_delta_height: Option<f32>,
    /// `[rollwiderstand]`: rolling resistance as a constant force, newtons (2,731 files).
    pub roll_resistance_n: Option<f32>,
    /// `[inv_min_turnradius]`: reciprocal of the smallest turning radius, 1/m (2,479 files).
    pub inv_min_turn_radius: Option<f32>,
    /// `[schwerpunkt]`: height of the centre of gravity, metres (1,908 files).
    pub centre_of_gravity_height: Option<f32>,
    /// `[rot_pnt_long]`: longitudinal coordinate of the rotation point, metres (2,731 files).
    pub rotation_point_long: Option<f32>,
    /// Moments of inertia about x (lateral), y (longitudinal) and z (vertical), t·m²: from
    /// `[momentofintertia]`, else from lines 2 to 4 of `[mass]` when those are not all zero.
    pub inertia_t_m2: Option<[f32; 3]>,
}

/// Parsed vehicle manifest configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct BusManifest {
    /// Friendly naming metadata.
    pub friendly_name: Option<FriendlyName>,
    /// Multi-line description text lines.
    pub description: Vec<String>,
    /// Path to fleet number `.org` list.
    pub number_file: Option<String>,
    /// Registration plate mode.
    pub registration_mode: RegistrationMode,
    /// Player audio configuration path (`sound.cfg`).
    pub sound_cfg: Option<String>,
    /// AI audio configuration path (`sound_ai.cfg`).
    pub sound_ai_cfg: Option<String>,
    /// 3D model configuration path (`model.cfg`).
    pub model_cfg: Option<String>,
    /// Passenger path network path (`paths.cfg`).
    pub paths_cfg: Option<String>,
    /// Passenger cabin layout path (`passengercabin.cfg`).
    pub passengercabin_cfg: Option<String>,
    /// Float variable definition files (`.txt` / `.var`).
    pub varnamelists: Vec<String>,
    /// String variable definition files.
    pub stringvarnamelists: Vec<String>,
    /// OMSI bytecode script files (`.osc`).
    pub scripts: Vec<String>,
    /// Constant parameter files.
    pub constfiles: Vec<String>,
    /// Whether this vehicle inherits scripts from the leading unit.
    pub script_share: bool,
    /// `[type]`: `None`/`0`/`1` road vehicle, `2` rail-bound, `3` aircraft.
    pub vehicle_type: Option<u32>,
    /// Half distance between the two bogies of a rail vehicle (`[boogies]`): the bogies sit at
    /// `±y` of the origin and the body pivots there.
    pub bogie_half_span: Option<f32>,
    /// Total curb vehicle weight in tonnes.
    pub mass_tonnes: Option<f32>,
    /// Primary bounding box (`[boundingbox]`): extents and centre offset in the vehicle frame
    /// (x lateral, y longitudinal, z up). The extents are full sizes (measured against plain
    /// body meshes), not half extents.
    pub bounding_box: Option<(glam::Vec3, glam::Vec3)>,
    /// Physical wheel axles.
    pub axles: Vec<BusAxle>,
    /// Front coupling joint Y position in meters (for articulated trailers).
    pub coupling_front_y: Option<f32>,
    /// Rear coupling joint Y position in meters (for articulated tractors).
    pub coupling_back_y: Option<f32>,
    /// Front coupling point `(x, y, z)` in the vehicle frame (`[coupling_front]`).
    pub coupling_front: Option<glam::Vec3>,
    /// Rear coupling point `(x, y, z)` in the vehicle frame (`[coupling_back]`).
    pub coupling_back: Option<glam::Vec3>,
    /// File name of the trailer coupled behind this vehicle (`[couple_back]`).
    pub couple_back: Option<String>,
    /// The trailer is coupled facing backwards (second line of `[couple_back]` is `true`): its
    /// *rear* coupling meets this vehicle's rear coupling, as the B head of the NF6D.
    pub couple_back_reversed: bool,
    /// Limits of the front coupling of a trailing segment (`[coupling_front_character]`).
    pub coupling_front_limits: Option<CouplingLimits>,
    /// Driver view camera presets.
    pub driver_cameras: Vec<BusCamera>,
    /// Passenger view camera presets.
    pub pax_cameras: Vec<BusCamera>,
    /// Mirror cameras in file order: entry N renders `reflexion<N>.bmp`.
    pub mirror_cameras: Vec<BusMirrorCamera>,
    /// Index of the default driver camera.
    pub std_camera_index: Option<usize>,
    /// Orbit pivot center for external exterior camera.
    pub outside_camera_center: Option<glam::Vec3>,
    /// AI and physics keywords beyond mass and axles.
    pub extras: VehicleExtras,
}

impl BusManifest {
    /// Parses a `.bus` vehicle manifest from text.
    ///
    /// # Errors
    /// Returns [`BusParseError`] if parameters cannot be parsed.
    pub fn parse(text: &str) -> Result<Self, BusParseError> {
        let blocks = scan_blocks(text);

        let mut friendly_name = None;
        let mut description = Vec::new();
        let mut number_file = None;
        let mut registration_mode = RegistrationMode::None;
        let mut sound_cfg = None;
        let mut sound_ai_cfg = None;
        let mut model_cfg = None;
        let mut paths_cfg = None;
        let mut passengercabin_cfg = None;
        let mut varnamelists = Vec::new();
        let mut stringvarnamelists = Vec::new();
        let mut scripts = Vec::new();
        let mut constfiles = Vec::new();
        let mut script_share = false;
        let mut mass_tonnes = None;
        let mut vehicle_type: Option<u32> = None;
        let mut bogie_half_span: Option<f32> = None;
        let mut bounding_box = None;
        let mut axles = Vec::new();
        let mut coupling_front_y = None;
        let mut coupling_back_y = None;
        let mut coupling_front = None;
        let mut coupling_back = None;
        let mut couple_back = None;
        let mut couple_back_reversed = false;
        let mut coupling_front_limits = None;
        let mut driver_cameras = Vec::new();
        let mut pax_cameras = Vec::new();
        let mut mirror_cameras = Vec::new();
        let mut std_camera_index = None;
        let mut outside_camera_center = None;
        let mut extras = VehicleExtras::default();
        let mut legacy_inertia: Option<[f32; 3]> = None;

        for block in blocks {
            match block.keyword.as_str() {
                "friendlyname" => {
                    if block.parameters.len() >= 3 {
                        friendly_name = Some(FriendlyName {
                            manufacturer: block.parameters[0].clone(),
                            model: block.parameters[1].clone(),
                            livery: block.parameters[2].clone(),
                        });
                    }
                }
                "description" => {
                    for line in block.parameters {
                        if line == "[end]" {
                            break;
                        }
                        description.push(line);
                    }
                }
                "number" => {
                    if let Some(first) = block.parameters.first() {
                        number_file = Some(first.clone());
                    }
                }
                "registration_list" => {
                    if let Some(first) = block.parameters.first() {
                        registration_mode = RegistrationMode::List(first.clone());
                    }
                }
                "registration_automatic" => {
                    if let Some(first) = block.parameters.first() {
                        registration_mode = RegistrationMode::Automatic(first.clone());
                    }
                }
                "registration_free" => {
                    registration_mode = RegistrationMode::Free;
                }
                "sound" => {
                    if let Some(first) = block.parameters.first() {
                        sound_cfg = Some(first.clone());
                    }
                }
                "sound_ai" => {
                    if let Some(first) = block.parameters.first() {
                        sound_ai_cfg = Some(first.clone());
                    }
                }
                "model" => {
                    if let Some(first) = block.parameters.first() {
                        model_cfg = Some(first.clone());
                    }
                }
                "paths" => {
                    if let Some(first) = block.parameters.first() {
                        paths_cfg = Some(first.clone());
                    }
                }
                "passengercabin" => {
                    if let Some(first) = block.parameters.first() {
                        passengercabin_cfg = Some(first.clone());
                    }
                }
                "varnamelist" => {
                    extract_counted_list(&block.parameters, &mut varnamelists);
                }
                "stringvarnamelist" => {
                    extract_counted_list(&block.parameters, &mut stringvarnamelists);
                }
                "script" => {
                    extract_counted_list(&block.parameters, &mut scripts);
                }
                "constfile" => {
                    extract_counted_list(&block.parameters, &mut constfiles);
                }
                "scriptshare" => {
                    script_share = true;
                }
                "boundingbox" => {
                    let v: Vec<f32> = block
                        .parameters
                        .iter()
                        .take(6)
                        .filter_map(|p| parse_float(p).ok())
                        .collect();
                    if v.len() == 6 {
                        bounding_box = Some((
                            glam::Vec3::new(v[0], v[1], v[2]),
                            glam::Vec3::new(v[3], v[4], v[5]),
                        ));
                    }
                }
                "type" => {
                    vehicle_type = block
                        .parameters
                        .first()
                        .and_then(|p| p.trim().parse::<u32>().ok());
                }
                "boogies" => {
                    bogie_half_span = block
                        .parameters
                        .first()
                        .and_then(|p| parse_float(p).ok())
                        .filter(|y| *y > 0.0);
                }
                "mass" => {
                    if let Some(first) = block.parameters.first() {
                        if let Ok(m) = parse_float(first) {
                            mass_tonnes = Some(m);
                        }
                    }
                    // Lines 2 to 4: the legacy moments of inertia (kept unless the modern block
                    // gives its own).
                    let legacy: Vec<f32> = block
                        .parameters
                        .iter()
                        .skip(1)
                        .take(3)
                        .map_while(|p| parse_float(p).ok())
                        .collect();
                    if let [x, y, z] = legacy[..] {
                        if (x, y, z) != (0.0, 0.0, 0.0) {
                            legacy_inertia = Some([x, y, z]);
                        }
                    }
                }
                "momentofintertia" => {
                    let v: Vec<f32> = block
                        .parameters
                        .iter()
                        .take(3)
                        .map_while(|p| parse_float(p).ok())
                        .collect();
                    if let [x, y, z] = v[..] {
                        extras.inertia_t_m2 = Some([x, y, z]);
                    }
                }
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
                    axles.push(axle);
                }
                "coupling_front" | "coupling_back" => {
                    // `[coupling_*]` carries `x, y, z`; the longitudinal position is the second.
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
                            coupling_front = Some(point);
                            coupling_front_y = Some(point.y);
                        } else {
                            coupling_back = Some(point);
                            coupling_back_y = Some(point.y);
                        }
                    } else if let [y] = v[..] {
                        // Hand-written files give only the longitudinal position.
                        if front {
                            coupling_front_y = Some(y);
                        } else {
                            coupling_back_y = Some(y);
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
                        coupling_front_limits = Some(CouplingLimits {
                            max_alpha_deg: a,
                            min_beta_deg: b0,
                            max_beta_deg: b1,
                            kind: k.max(0.0) as u32,
                        });
                    }
                }
                "couple_back" => {
                    couple_back = block
                        .parameters
                        .first()
                        .map(|f| f.trim().to_string())
                        .filter(|f| !f.is_empty());
                    couple_back_reversed = block
                        .parameters
                        .get(1)
                        .is_some_and(|f| f.trim().eq_ignore_ascii_case("true"));
                }
                "add_camera_driver" if block.parameters.len() >= 7 => {
                    if let Some(cam) = parse_camera_block(&block.parameters, true) {
                        driver_cameras.push(cam);
                    }
                }
                "add_camera_pax" if block.parameters.len() >= 7 => {
                    if let Some(cam) = parse_camera_block(&block.parameters, false) {
                        pax_cameras.push(cam);
                    }
                }
                "add_camera_reflexion" | "add_camera_reflexion_2" => {
                    if let Some(cam) = parse_mirror_block(&block.parameters) {
                        mirror_cameras.push(cam);
                    }
                }
                "set_camera_std" => {
                    if let Some(first) = block.parameters.first() {
                        std_camera_index = first.trim().parse::<usize>().ok();
                    }
                }
                "set_camera_outside_center" if block.parameters.len() >= 3 => {
                    let x = parse_float(&block.parameters[0]).unwrap_or(0.0);
                    let y = parse_float(&block.parameters[1]).unwrap_or(0.0);
                    let z = parse_float(&block.parameters[2]).unwrap_or(0.0);
                    outside_camera_center = Some(glam::Vec3::new(x, y, z));
                }
                "ai_veh_type" => {
                    extras.ai_veh_type = block
                        .parameters
                        .first()
                        .and_then(|p| p.trim().parse::<u32>().ok());
                }
                "ai_brakeperformance" => {
                    extras.ai_brake_performance = block
                        .parameters
                        .iter()
                        .map_while(|p| parse_float(p).ok())
                        .collect();
                }
                "rowdy_factor" => {
                    let v: Vec<f32> = block
                        .parameters
                        .iter()
                        .map_while(|p| parse_float(p).ok())
                        .take(2)
                        .collect();
                    if let [lo, hi] = v[..] {
                        extras.rowdy_factor = Some((lo.min(hi), lo.max(hi)));
                    }
                }
                "ai_deltaheight" => extras.ai_delta_height = first_float(&block.parameters),
                "rollwiderstand" => extras.roll_resistance_n = first_float(&block.parameters),
                "inv_min_turnradius" => {
                    extras.inv_min_turn_radius = first_float(&block.parameters);
                }
                "schwerpunkt" => {
                    extras.centre_of_gravity_height = first_float(&block.parameters);
                }
                "rot_pnt_long" => extras.rotation_point_long = first_float(&block.parameters),
                "view_schedule" => {
                    if let Some(cam) = driver_cameras.last_mut() {
                        cam.is_schedule_view = true;
                    }
                }
                "view_ticketselling" => {
                    if let Some(cam) = driver_cameras.last_mut() {
                        cam.is_ticket_view = true;
                    }
                }
                _ => {}
            }
        }

        if extras.inertia_t_m2.is_none() {
            extras.inertia_t_m2 = legacy_inertia;
        }

        Ok(Self {
            friendly_name,
            description,
            number_file,
            registration_mode,
            sound_cfg,
            sound_ai_cfg,
            model_cfg,
            paths_cfg,
            passengercabin_cfg,
            varnamelists,
            stringvarnamelists,
            scripts,
            constfiles,
            script_share,
            mass_tonnes,
            vehicle_type,
            bogie_half_span,
            bounding_box,
            axles,
            coupling_front_y,
            coupling_back_y,
            coupling_front,
            coupling_back,
            couple_back,
            couple_back_reversed,
            coupling_front_limits,
            driver_cameras,
            pax_cameras,
            mirror_cameras,
            std_camera_index,
            outside_camera_center,
            extras,
        })
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

mod blocks;
use blocks::{
    extract_counted_list, first_float, parse_axle_block, parse_camera_block, parse_mirror_block,
};

/// Convenience function to parse a `.bus` file from a string slice.
pub fn parse_bus(text: &str) -> Result<BusManifest, BusParseError> {
    BusManifest::parse(text)
}

#[cfg(test)]
mod tests;
