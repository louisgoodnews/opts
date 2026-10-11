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
    /// Limits of the front coupling of a trailing segment (`[coupling_front_character]`).
    pub coupling_front_limits: Option<CouplingLimits>,
    /// Driver view camera presets.
    pub driver_cameras: Vec<BusCamera>,
    /// Passenger view camera presets.
    pub pax_cameras: Vec<BusCamera>,
    /// Index of the default driver camera.
    pub std_camera_index: Option<usize>,
    /// Orbit pivot center for external exterior camera.
    pub outside_camera_center: Option<glam::Vec3>,
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
        let mut bounding_box = None;
        let mut axles = Vec::new();
        let mut coupling_front_y = None;
        let mut coupling_back_y = None;
        let mut coupling_front = None;
        let mut coupling_back = None;
        let mut couple_back = None;
        let mut coupling_front_limits = None;
        let mut driver_cameras = Vec::new();
        let mut pax_cameras = Vec::new();
        let mut std_camera_index = None;
        let mut outside_camera_center = None;

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
                "mass" => {
                    if let Some(first) = block.parameters.first() {
                        if let Ok(m) = parse_float(first) {
                            mass_tonnes = Some(m);
                        }
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
            bounding_box,
            axles,
            coupling_front_y,
            coupling_back_y,
            coupling_front,
            coupling_back,
            couple_back,
            coupling_front_limits,
            driver_cameras,
            pax_cameras,
            std_camera_index,
            outside_camera_center,
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

fn extract_counted_list(params: &[String], dest: &mut Vec<String>) {
    if params.is_empty() {
        return;
    }
    // Line 1 is the declared count N
    let count = params[0].parse::<usize>().unwrap_or(params.len() - 1);
    for item in params.iter().skip(1).take(count) {
        dest.push(item.clone());
    }
}

fn parse_axle_block(params: &[String], axle: &mut BusAxle) {
    let mut iter = params.iter();
    while let Some(ident) = iter.next() {
        match ident.as_str() {
            "achse_long" => {
                if let Some(val) = iter.next() {
                    axle.longitudinal_m = parse_float(val).unwrap_or(0.0);
                }
            }
            "achse_maxwidth" => {
                if let Some(val) = iter.next() {
                    axle.max_width_m = parse_float(val).ok();
                }
            }
            "achse_minwidth" => {
                if let Some(val) = iter.next() {
                    axle.min_width_m = parse_float(val).ok();
                }
            }
            "achse_raddurchmesser" => {
                if let Some(val) = iter.next() {
                    axle.wheel_diameter_m = parse_float(val).ok();
                }
            }
            "achse_feder" => {
                if let Some(val) = iter.next() {
                    axle.spring_rate = parse_float(val).ok();
                }
            }
            "achse_maxforce" => {
                if let Some(val) = iter.next() {
                    axle.max_force = parse_float(val).ok();
                }
            }
            "achse_daempfer" => {
                if let Some(val) = iter.next() {
                    axle.damping = parse_float(val).ok();
                }
            }
            "achse_antrieb" => {
                if let Some(val) = iter.next() {
                    axle.is_driven = val == "1" || val.eq_ignore_ascii_case("true");
                }
            }
            _ => {}
        }
    }
}

fn parse_camera_block(params: &[String], is_driver: bool) -> Option<BusCamera> {
    if params.len() < 7 {
        return None;
    }
    let x = parse_float(&params[0]).ok()?;
    let y = parse_float(&params[1]).ok()?;
    let z = parse_float(&params[2]).ok()?;
    let dist = parse_float(&params[3]).ok()?;
    let fov = parse_float(&params[4]).ok()?;
    let yaw = parse_float(&params[5]).ok()?;
    let pitch = parse_float(&params[6]).ok()?;

    Some(BusCamera {
        x,
        y,
        z,
        dist,
        fov_deg: fov,
        yaw_deg: yaw,
        pitch_deg: pitch,
        is_driver,
        is_schedule_view: false,
        is_ticket_view: false,
    })
}

/// Convenience function to parse a `.bus` file from a string slice.
pub fn parse_bus(text: &str) -> Result<BusManifest, BusParseError> {
    BusManifest::parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupling_front_character_is_parsed() {
        let m = BusManifest::parse(
            "[coupling_front]\n0\n4.5\n0.3\n\n[coupling_front_character]\n52.5\n-20\n20\n1\n",
        )
        .unwrap();
        let l = m.coupling_front_limits.expect("limits");
        assert_eq!(
            (l.max_alpha_deg, l.min_beta_deg, l.max_beta_deg, l.kind),
            (52.5, -20.0, 20.0, 1)
        );
        assert_eq!(m.coupling_front.map(|p| p.y), Some(4.5));
    }

    #[test]
    fn test_parse_sample_bus() {
        let sample = r#"
[friendlyname]
MAN
NL202
BVG Standard

[description]
Standard Berliner Linienbus
Baujahr 1992
[end]

[number]
Regs_NL202.org

[sound]
sound\sound_NL202.cfg

[paths]
model\paths_NL202.cfg

[passengercabin]
model\passengercabin_NL202.cfg

[model]
model\model_NL202.cfg

[varnamelist]
2
script\engine_varlist.txt
script\door_varlist.txt

[script]
1
script\engine.osc

[constfile]
1
script\engine_constfile.txt

[mass]
11.5

[newachse]
achse_long
3.238
achse_maxwidth
2.25
achse_antrieb
1
"#;
        let bus = parse_bus(sample).unwrap();
        assert_eq!(
            bus.friendly_name,
            Some(FriendlyName {
                manufacturer: "MAN".to_string(),
                model: "NL202".to_string(),
                livery: "BVG Standard".to_string(),
            })
        );
        assert_eq!(
            bus.description,
            vec!["Standard Berliner Linienbus", "Baujahr 1992"]
        );
        assert_eq!(bus.number_file, Some("Regs_NL202.org".to_string()));
        assert_eq!(bus.sound_cfg, Some(r"sound\sound_NL202.cfg".to_string()));
        assert_eq!(bus.paths_cfg, Some(r"model\paths_NL202.cfg".to_string()));
        assert_eq!(bus.varnamelists.len(), 2);
        assert_eq!(bus.scripts.len(), 1);
        assert_eq!(bus.constfiles.len(), 1);
        assert_eq!(bus.mass_tonnes, Some(11.5));
        assert_eq!(bus.axles.len(), 1);
        assert_eq!(bus.axles[0].longitudinal_m, 3.238);
        assert_eq!(bus.axles[0].max_width_m, Some(2.25));
        assert!(bus.axles[0].is_driven);
    }

    #[test]
    fn test_parse_bus_cameras() {
        let sample = r#"
[add_camera_driver]
-0.70
4.40
1.80
-0.06
55.0
0.0
-5.0

[view_schedule]

[add_camera_driver]
-0.70
4.40
1.80
-0.06
55.0
-45.0
-15.0

[view_ticketselling]

[set_camera_std]
0

[add_camera_pax]
0.0
2.0
1.7
0.0
65.0
0.0
0.0

[set_camera_outside_center]
0.0
0.5
1.5
"#;
        let bus = parse_bus(sample).unwrap();
        assert_eq!(bus.driver_cameras.len(), 2);
        assert_eq!(bus.driver_cameras[0].x, -0.70);
        assert_eq!(bus.driver_cameras[0].y, 4.40);
        assert_eq!(bus.driver_cameras[0].z, 1.80);
        assert_eq!(bus.driver_cameras[0].dist, -0.06);
        assert_eq!(bus.driver_cameras[0].fov_deg, 55.0);
        assert_eq!(bus.driver_cameras[0].yaw_deg, 0.0);
        assert_eq!(bus.driver_cameras[0].pitch_deg, -5.0);
        assert!(bus.driver_cameras[0].is_driver);
        assert!(bus.driver_cameras[0].is_schedule_view);
        assert!(!bus.driver_cameras[0].is_ticket_view);

        assert_eq!(bus.driver_cameras[1].yaw_deg, -45.0);
        assert!(bus.driver_cameras[1].is_ticket_view);

        assert_eq!(bus.std_camera_index, Some(0));

        assert_eq!(bus.pax_cameras.len(), 1);
        assert_eq!(bus.pax_cameras[0].y, 2.0);
        assert!(!bus.pax_cameras[0].is_driver);

        assert_eq!(
            bus.outside_camera_center,
            Some(glam::Vec3::new(0.0, 0.5, 1.5))
        );
    }
}
