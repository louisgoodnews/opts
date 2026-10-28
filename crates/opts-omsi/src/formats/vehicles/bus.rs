//! Parser and data model for OMSI vehicle manifests (`.bus`).
//!
//! A `.bus` file is the central manifest for a drivable or AI vehicle in OMSI.
//! It declares identity metadata, references to model, sound, and path files,
//! the script system files, physical properties (mass, axles), and coupling links.
//!
//! See specification in `docs/src/formats/text/vehicles/bus.md`.

use super::error::BusParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks, OmsiBlock};

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

/// One `[control_cable_front]` / `[control_cable_back]` entry: a pair of script variables that
/// travel between coupled units (the tram's battery switch, indicators, doors, traction demand).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCable {
    /// The cable sits at the unit's front coupling (else the rear one).
    pub front: bool,
    /// Plug kind: `C` (central, links to the other unit's `C` of the same number), `L` or `R`.
    pub kind: char,
    /// Number of the cable within its kind.
    pub index: u32,
    /// The variable that receives the value the cable carries (the scripts read it).
    pub receive: String,
    /// The variable the unit's scripts write to put a value on the cable.
    pub send: String,
    /// The link works only while this variable of the unit is above 0.5.
    pub coupling: String,
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
    /// `[control_cable_front]` and `[control_cable_back]` entries in file order.
    pub control_cables: Vec<ControlCable>,
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

impl VehicleExtras {
    pub(super) fn apply_block(&mut self, block: &OmsiBlock) -> bool {
        match block.keyword.as_str() {
            "momentofintertia" => {
                let v: Vec<f32> = block
                    .parameters
                    .iter()
                    .take(3)
                    .map_while(|p| parse_float(p).ok())
                    .collect();
                if let [x, y, z] = v[..] {
                    self.inertia_t_m2 = Some([x, y, z]);
                }
            }
            "ai_veh_type" => {
                self.ai_veh_type = block
                    .parameters
                    .first()
                    .and_then(|p| p.trim().parse::<u32>().ok());
            }
            "ai_brakeperformance" => {
                self.ai_brake_performance = block
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
                    self.rowdy_factor = Some((lo.min(hi), lo.max(hi)));
                }
            }
            "ai_deltaheight" => self.ai_delta_height = blocks::first_float(&block.parameters),
            "rollwiderstand" => self.roll_resistance_n = blocks::first_float(&block.parameters),
            "inv_min_turnradius" => {
                self.inv_min_turn_radius = blocks::first_float(&block.parameters);
            }
            "schwerpunkt" => {
                self.centre_of_gravity_height = blocks::first_float(&block.parameters);
            }
            tag @ ("control_cable_front" | "control_cable_back") => {
                if let [kind, index, receive, send, coupling, ..] = &block.parameters[..] {
                    if let (Some(kind), Ok(index)) =
                        (kind.trim().chars().next(), index.trim().parse::<u32>())
                    {
                        self.control_cables.push(ControlCable {
                            front: tag == "control_cable_front",
                            kind: kind.to_ascii_uppercase(),
                            index,
                            receive: receive.trim().to_owned(),
                            send: send.trim().to_owned(),
                            coupling: coupling.trim().to_owned(),
                        });
                    }
                }
            }
            "rot_pnt_long" => self.rotation_point_long = blocks::first_float(&block.parameters),
            _ => return false,
        }
        true
    }
}

mod blocks;
mod parse;

/// Convenience function to parse a `.bus` file from a string slice.
pub fn parse_bus(text: &str) -> Result<BusManifest, BusParseError> {
    BusManifest::parse(text)
}

#[cfg(test)]
mod tests;
