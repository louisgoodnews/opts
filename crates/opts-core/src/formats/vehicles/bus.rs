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
    /// Physical wheel axles.
    pub axles: Vec<BusAxle>,
    /// Front coupling joint Y position in meters (for articulated trailers).
    pub coupling_front_y: Option<f32>,
    /// Rear coupling joint Y position in meters (for articulated tractors).
    pub coupling_back_y: Option<f32>,
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
        let mut axles = Vec::new();
        let mut coupling_front_y = None;
        let mut coupling_back_y = None;

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
                "coupling_front" => {
                    if let Some(first) = block.parameters.first() {
                        coupling_front_y = parse_float(first).ok();
                    }
                }
                "coupling_back" => {
                    if let Some(first) = block.parameters.first() {
                        coupling_back_y = parse_float(first).ok();
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
            axles,
            coupling_front_y,
            coupling_back_y,
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

/// Convenience function to parse a `.bus` file from a string slice.
pub fn parse_bus(text: &str) -> Result<BusManifest, BusParseError> {
    BusManifest::parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
