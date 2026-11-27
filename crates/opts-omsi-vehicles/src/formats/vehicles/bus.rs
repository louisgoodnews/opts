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
    /// File name of the unit coupled in front of this vehicle (`[couple_front]`, reversed units).
    /// Read only: the consist planner still walks `[couple_back]` alone (ISS-075).
    pub couple_front: Option<String>,
    /// The front unit is coupled facing backwards (second line of `[couple_front]` is `true`).
    pub couple_front_reversed: bool,
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

mod blocks;
mod extras;
mod parse;

pub use extras::{ControlCable, VehicleExtras};

/// Convenience function to parse a `.bus` file from a string slice.
pub fn parse_bus(text: &str) -> Result<BusManifest, BusParseError> {
    BusManifest::parse(text)
}
