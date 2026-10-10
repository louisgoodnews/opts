//! Parser and data structures for OMSI Scenery Object (`.sco`) files.
//!
//! A `.sco` file defines a scenery object (building, tree, street lamp, junction,
//! traffic light, etc.) placed on map tiles. It specifies 3D mesh attachments,
//! materials, collision geometry, traffic paths, lights, and scripting hooks.
//!
//! See specification in `docs/src/formats/text/sco.md`.

use glam::Vec3;

use super::error::SceneryParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks, OmsiBlock};

/// Billboard tree helper (`[tree]`): texture plus default height and width-ratio ranges.
///
/// Placed instances override the defaults through their per-instance string variables
/// (texture, height in metres, width/height ratio).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTree {
    /// Billboard texture file.
    pub texture: String,
    /// Minimum height in metres.
    pub min_height: f32,
    /// Maximum height in metres.
    pub max_height: f32,
    /// Minimum width/height ratio.
    pub min_width_ratio: f32,
    /// Maximum width/height ratio.
    pub max_width_ratio: f32,
}

/// Bounding box of a scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoBoundingBox {
    /// Half extents in meters along local X, Y, and Z axes.
    pub half_extents: Vec3,
    /// Center offset in meters along local X, Y, and Z axes.
    pub center: Vec3,
}

/// Material properties attached to a mesh in a scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMaterial {
    /// Texture bitmap filename.
    pub texture: String,
    /// Material slot index in the 3D mesh.
    pub material_index: usize,
    /// Alpha transparency mode (0 = opaque, 1 = alpha test, 2 = alpha blend).
    pub alpha_mode: Option<u32>,
    /// Environment reflection map filename and reflection strength factor (0.0..1.0).
    pub env_map: Option<(String, f32)>,
    /// Whether the environment map is masked by alpha.
    pub env_map_mask: bool,
    /// Lightmap bitmap filename.
    pub light_map: Option<String>,
    /// Night texture filename.
    pub night_map: Option<String>,
    /// Whether material transparency scales with `[alphascale]`.
    pub trans_map: bool,
    /// Disable writing to depth buffer (Z-buffer).
    pub no_z_write: bool,
    /// Disable depth testing.
    pub no_z_check: bool,
    /// Free runtime texture binding variable name.
    pub free_tex: Option<String>,
    /// Dynamic material texture change: `(texture, material_index, variable_name)`.
    pub matl_change: Option<(String, usize, String)>,
    /// Variable name driving material alpha scale.
    pub alpha_scale: Option<String>,
}

impl ScoMaterial {
    /// Creates a new material with the specified texture and slot index.
    #[must_use]
    pub fn new(texture: impl Into<String>, material_index: usize) -> Self {
        Self {
            texture: texture.into(),
            material_index,
            alpha_mode: None,
            env_map: None,
            env_map_mask: false,
            light_map: None,
            night_map: None,
            trans_map: false,
            no_z_write: false,
            no_z_check: false,
            free_tex: None,
            matl_change: None,
            alpha_scale: None,
        }
    }
}

/// Animation definition block (`[newanim]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoAnimation {
    /// Animation sub-commands and their string parameters.
    pub commands: Vec<(String, Vec<String>)>,
}

/// 3D mesh attachment in a scenery object (`[mesh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMesh {
    /// Relative path to `.o3d` or `.x` mesh file.
    pub file: String,
    /// Level-of-detail threshold under which this mesh is active.
    pub lod_threshold: Option<f32>,
    /// Whether this specific mesh casts shadows.
    pub cast_shadow: Option<bool>,
    /// Materials overriding or extending this mesh.
    pub materials: Vec<ScoMaterial>,
    /// Conditional visibility: `(variable_name, expected_value)`.
    pub visible: Option<(String, f32)>,
    /// Interaction mouse click event trigger name.
    pub mouse_event: Option<String>,
    /// Text texture slots bound to this mesh (`[useTextTexture]`).
    pub text_texture_indices: Vec<usize>,
    /// Animation definitions attached to this mesh.
    pub animations: Vec<ScoAnimation>,
}

impl ScoMesh {
    /// Creates a new mesh attachment with the specified file path.
    #[must_use]
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            lod_threshold: None,
            cast_shadow: None,
            materials: Vec::new(),
            visible: None,
            mouse_event: None,
            text_texture_indices: Vec::new(),
            animations: Vec::new(),
        }
    }
}

/// Traffic light phase program definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoPhase {
    /// Signal phase state code (e.g. 0 = red, 6 = green, 9 = yellow).
    pub state: u32,
    /// Phase duration in seconds.
    pub duration_sec: f32,
}

/// Named traffic light signal program.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTrafficLightProgram {
    /// Program name (e.g. "Main", "RPB Haupt").
    pub name: String,
    /// Sequential phase timing entries.
    pub phases: Vec<ScoPhase>,
}

/// Traffic light controller group.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTrafficLightGroup {
    /// Cycle time in seconds (e.g. 99.99).
    pub cycle_time_sec: f32,
    /// Signal programs belonging to this group.
    pub programs: Vec<ScoTrafficLightProgram>,
}

/// Map illumination light source (`[maplight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMapLight {
    /// Light source position in object-local meters.
    pub position: Vec3,
    /// Light color RGB (0.0..1.0).
    pub color: Vec3,
    /// Radius at full intensity in meters.
    pub radius: f32,
}

/// Traffic path lane segment for AI road traffic, pedestrians, or rail (`[path]`, `[path_2]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoPath {
    /// Start position in object-local coordinates.
    pub start: Vec3,
    /// Initial heading in degrees.
    pub heading_deg: f32,
    /// Curve radius in meters (large negative values indicate straight segment).
    pub radius_m: f32,
    /// Arc length in meters.
    pub length_m: f32,
    /// Starting gradient/slope.
    pub slope_start: f32,
    /// Ending gradient/slope.
    pub slope_end: f32,
    /// Path traffic type: 0 = road vehicles, 1 = pedestrians, 2 = rail.
    pub path_type: u32,
    /// Lane width in meters.
    pub width_m: f32,
    /// Allowed travel direction: 0 = forward only, 1 = reverse only, 2 = bidirectional.
    pub direction: u32,
    /// Extra flag or subtype.
    pub extra: Option<u32>,
    /// Index of traffic light controlling this path (if governed by `[use_traffic_light]`).
    pub traffic_light_index: Option<usize>,
    /// Distance in meters before path at which vehicles evaluate traffic light (`[approachdist]`).
    pub approach_distance_m: Option<f32>,
}

/// Spline helper junction arm (`[splinehelper]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoSplineHelper {
    /// Spline profile filename (`.sli`).
    pub profile_file: String,
    /// Position in object-local space.
    pub position: Vec3,
    /// Rotation in degrees.
    pub rotation_deg: f32,
    /// Additional numeric parameters.
    pub extra: Vec<f32>,
}

/// Text texture configuration (`[texttexture]`, `[texttexture_enh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTextTexture {
    /// Source string variable index (`0` when the source is named, see `string_var`).
    pub string_index: usize,
    /// Source string variable *name* (`[texttexture_enh]` addresses its string by name,
    /// e.g. `DFI_Spalte_Linie1`).
    pub string_var: Option<String>,
    /// Font name.
    pub font_name: String,
    /// Rendered texture width in pixels.
    pub width: u32,
    /// Rendered texture height in pixels.
    pub height: u32,
    /// Color mode (0 = monochrome, 1 = keep colors).
    pub color_mode: u32,
    /// Text color RGB (0..255).
    pub text_color: [u8; 3],
}

/// Change-The-Color (CTC) repaint picker configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoCtc {
    /// Variable name receiving the selection.
    pub var_name: String,
    /// Relative directory containing livery textures.
    pub texture_dir: String,
    /// Default selection index.
    pub default_index: usize,
}

/// Complete representation of an OMSI scenery object (`.sco`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneryObject {
    /// Editor group category hierarchy (e.g. `["Aachen", "Ampeln"]`).
    pub groups: Vec<String>,
    /// Human-readable friendly name in editor.
    pub friendly_name: Option<String>,
    /// Complexity draw-priority classification (0 = very important .. 3 = detail).
    pub complexity: Option<u32>,
    /// Billboard tree definition (`[tree]`), if any.
    pub tree: Option<ScoTree>,
    /// Whether this object is only visible in the editor.
    pub only_editor: bool,
    /// Whether this object can be joined with neighbors in the editor.
    pub joinable: bool,
    /// Whether this object is a route helper arrow.
    pub is_help_arrow: bool,
    /// Whether this object is anchored to the terrain grid.
    pub fixed: bool,
    /// Whether elevation values are absolute rather than terrain-relative.
    pub abs_height: bool,
    /// Whether this object acts as a drivable ground surface.
    pub surface: bool,
    /// Global collision disabled flag.
    pub no_collision: bool,
    /// Global shadow disabled flag.
    pub no_shadow: bool,
    /// Disable map lighting on this object.
    pub no_map_lighting: bool,
    /// Render pass designation (`"0"`, `"surface"`, etc.).
    pub render_type: Option<String>,
    /// Night lighting rule classification (0 = street, 1 = continuous, 2 = residential, etc.).
    pub night_map_mode: Option<u32>,
    /// Whether mesh materials use lightmap UV mappings.
    pub light_map_mapping: bool,
    /// Distance culling size factor.
    pub detail_factor: Option<f32>,
    /// Never cull by distance.
    pub no_distance_check: bool,
    /// Marks this object as a traffic light head.
    pub is_traffic_light: bool,

    /// Collision meshes attached to the object.
    pub collision_meshes: Vec<String>,
    /// Bounding box.
    pub bounding_box: Option<ScoBoundingBox>,
    /// Center of gravity.
    pub cog: Option<Vec3>,
    /// Breakable pole physics parameters: (stiffness, factor).
    pub crashmode_pole: Option<(f32, f32)>,

    /// Visual 3D mesh attachments.
    pub meshes: Vec<ScoMesh>,
    /// Traffic path lane segments.
    pub paths: Vec<ScoPath>,
    /// Spline helper profiles.
    pub spline_helpers: Vec<ScoSplineHelper>,
    /// Map light sources.
    pub map_lights: Vec<ScoMapLight>,
    /// Traffic light controller groups.
    pub traffic_light_groups: Vec<ScoTrafficLightGroup>,
    /// Text texture render definitions.
    pub text_textures: Vec<ScoTextTexture>,
    /// Change-the-color configuration.
    pub ctc: Option<ScoCtc>,
    /// CTC texture catalog entries (label, file).
    pub ctc_textures: Vec<(String, String)>,

    /// Bytecode scripts (`.osc`).
    pub scripts: Vec<String>,
    /// Variable definition files (`_varlist.txt`).
    pub varnamelists: Vec<String>,
    /// String variable definition files (`_stringvarlist.txt`).
    pub stringvarnamelists: Vec<String>,
    /// Constant parameter files (`_constfile.txt`).
    pub constfiles: Vec<String>,
    /// Sound configuration file (`sound.cfg`).
    pub sound: Option<String>,
    /// Passenger cabin layout file (`passengercabin.cfg`).
    pub passenger_cabin: Option<String>,
}

impl SceneryObject {
    /// Parses a scenery object from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, SceneryParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a scenery object from decoded text.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, SceneryParseError> {
        let blocks = scan_blocks(text);
        let mut obj = SceneryObject::default();

        let mut current_lod: Option<f32> = None;
        let mut active_traffic_light: Option<usize> = None;
        let mut active_approach_dist: Option<f32> = None;

        for block in &blocks {
            match block.keyword.as_str() {
                "groups" => {
                    if let Some(first) = block.parameters.first() {
                        let count = parse_usize(first, "groups count", block.line_number)?;
                        obj.groups = block
                            .parameters
                            .iter()
                            .skip(1)
                            .take(count)
                            .cloned()
                            .collect();
                    }
                }
                "friendlyname" => {
                    if let Some(name) = block.parameters.first() {
                        obj.friendly_name = Some(name.clone());
                    }
                }
                "complexity" => {
                    if let Some(val) = block.parameters.first() {
                        obj.complexity = Some(parse_u32(val, "complexity", block.line_number)?);
                    }
                }
                "tree" => {
                    if block.parameters.len() >= 5 {
                        obj.tree = Some(ScoTree {
                            texture: block.parameters[0].clone(),
                            min_height: parse_f32(
                                &block.parameters[1],
                                "tree min height",
                                block.line_number,
                            )?,
                            max_height: parse_f32(
                                &block.parameters[2],
                                "tree max height",
                                block.line_number,
                            )?,
                            min_width_ratio: parse_f32(
                                &block.parameters[3],
                                "tree min width ratio",
                                block.line_number,
                            )?,
                            max_width_ratio: parse_f32(
                                &block.parameters[4],
                                "tree max width ratio",
                                block.line_number,
                            )?,
                        });
                    }
                }
                "onlyeditor" => {
                    obj.only_editor = true;
                }
                "joinable" => {
                    obj.joinable = true;
                }
                "helparrow" => {
                    obj.is_help_arrow = true;
                }
                "fixed" => {
                    obj.fixed = true;
                }
                "absheight" => {
                    obj.abs_height = true;
                }
                "surface" => {
                    obj.surface = true;
                }
                "nocollision" => {
                    obj.no_collision = true;
                }
                "noshadow" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        mesh.cast_shadow = Some(false);
                    } else {
                        obj.no_shadow = true;
                    }
                }
                "shadow" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        mesh.cast_shadow = Some(true);
                    }
                }
                "nomaplighting" => {
                    obj.no_map_lighting = true;
                }
                "rendertype" => {
                    if let Some(val) = block.parameters.first() {
                        obj.render_type = Some(val.clone());
                    }
                }
                "nightmapmode" => {
                    if let Some(val) = block.parameters.first() {
                        obj.night_map_mode =
                            Some(parse_u32(val, "NightMapMode", block.line_number)?);
                    }
                }
                "lightmapmapping" => {
                    obj.light_map_mapping = true;
                }
                "detail_factor" => {
                    if let Some(val) = block.parameters.first() {
                        obj.detail_factor =
                            Some(parse_f32(val, "detail_factor", block.line_number)?);
                    }
                }
                "nodistancecheck" => {
                    obj.no_distance_check = true;
                }
                "trafficlight" => {
                    obj.is_traffic_light = true;
                }
                "lod" => {
                    if let Some(val) = block.parameters.first() {
                        let threshold = parse_f32(val, "LOD", block.line_number)?;
                        current_lod = Some(threshold);
                    }
                }
                "mesh" => {
                    if let Some(file) = block.parameters.first() {
                        let mut mesh = ScoMesh::new(file.clone());
                        mesh.lod_threshold = current_lod;
                        obj.meshes.push(mesh);
                    }
                }
                "matl" => {
                    if block.parameters.len() >= 2 {
                        let tex = &block.parameters[0];
                        let mat_idx =
                            parse_usize(&block.parameters[1], "matl index", block.line_number)?;
                        let mat = ScoMaterial::new(tex.clone(), mat_idx);
                        if let Some(mesh) = obj.meshes.last_mut() {
                            mesh.materials.push(mat);
                        }
                    }
                }
                "matl_alpha" => {
                    if let Some(val) = block.parameters.first() {
                        let mode = parse_u32(val, "matl_alpha", block.line_number)?;
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.alpha_mode = Some(mode);
                            }
                        }
                    }
                }
                "matl_envmap" => {
                    if block.parameters.len() >= 2 {
                        let env_file = block.parameters[0].clone();
                        let strength = parse_f32(
                            &block.parameters[1],
                            "matl_envmap strength",
                            block.line_number,
                        )?;
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.env_map = Some((env_file, strength));
                            }
                        }
                    }
                }
                "matl_envmap_mask" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        if let Some(mat) = mesh.materials.last_mut() {
                            mat.env_map_mask = true;
                        }
                    }
                }
                "matl_lightmap" => {
                    if let Some(val) = block.parameters.first() {
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.light_map = Some(val.clone());
                            }
                        }
                    }
                }
                "matl_nightmap" => {
                    if let Some(val) = block.parameters.first() {
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.night_map = Some(val.clone());
                            }
                        }
                    }
                }
                "matl_transmap" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        if let Some(mat) = mesh.materials.last_mut() {
                            mat.trans_map = true;
                        }
                    }
                }
                "matl_nozwrite" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        if let Some(mat) = mesh.materials.last_mut() {
                            mat.no_z_write = true;
                        }
                    }
                }
                "matl_nozcheck" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        if let Some(mat) = mesh.materials.last_mut() {
                            mat.no_z_check = true;
                        }
                    }
                }
                "matl_freetex" => {
                    if let Some(val) = block.parameters.first() {
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.free_tex = Some(val.clone());
                            }
                        }
                    }
                }
                "matl_change" => {
                    if block.parameters.len() >= 3 {
                        let tex = block.parameters[0].clone();
                        let idx = parse_usize(
                            &block.parameters[1],
                            "matl_change index",
                            block.line_number,
                        )?;
                        let var = block.parameters[2].clone();
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.matl_change = Some((tex, idx, var));
                            }
                        }
                    }
                }
                "alphascale" => {
                    if let Some(val) = block.parameters.first() {
                        if let Some(mesh) = obj.meshes.last_mut() {
                            if let Some(mat) = mesh.materials.last_mut() {
                                mat.alpha_scale = Some(val.clone());
                            }
                        }
                    }
                }
                "visible" => {
                    if block.parameters.len() >= 2 {
                        let var = block.parameters[0].clone();
                        let val =
                            parse_f32(&block.parameters[1], "visible value", block.line_number)?;
                        if let Some(mesh) = obj.meshes.last_mut() {
                            mesh.visible = Some((var, val));
                        }
                    }
                }
                "mouseevent" => {
                    if let Some(val) = block.parameters.first() {
                        if let Some(mesh) = obj.meshes.last_mut() {
                            mesh.mouse_event = Some(val.clone());
                        }
                    }
                }
                "usetexttexture" => {
                    if let Some(val) = block.parameters.first() {
                        let idx = parse_usize(val, "useTextTexture index", block.line_number)?;
                        if let Some(mesh) = obj.meshes.last_mut() {
                            mesh.text_texture_indices.push(idx);
                        }
                    }
                }
                "newanim" => {
                    if let Some(mesh) = obj.meshes.last_mut() {
                        let anim = parse_anim_block(block)?;
                        mesh.animations.push(anim);
                    }
                }
                "collision_mesh" | "collisionsmesh" | "collisionmesh" | "collisioionsmesh" => {
                    if let Some(file) = block.parameters.first() {
                        obj.collision_meshes.push(file.clone());
                    }
                }
                "boundingbox" => {
                    if block.parameters.len() >= 6 {
                        let hx = parse_f32(
                            &block.parameters[0],
                            "boundingbox half_x",
                            block.line_number,
                        )?;
                        let hy = parse_f32(
                            &block.parameters[1],
                            "boundingbox half_y",
                            block.line_number,
                        )?;
                        let hz = parse_f32(
                            &block.parameters[2],
                            "boundingbox half_z",
                            block.line_number,
                        )?;
                        let cx =
                            parse_f32(&block.parameters[3], "boundingbox cx", block.line_number)?;
                        let cy =
                            parse_f32(&block.parameters[4], "boundingbox cy", block.line_number)?;
                        let cz =
                            parse_f32(&block.parameters[5], "boundingbox cz", block.line_number)?;
                        obj.bounding_box = Some(ScoBoundingBox {
                            half_extents: Vec3::new(hx, hy, hz),
                            center: Vec3::new(cx, cy, cz),
                        });
                    }
                }
                "cog" => {
                    if block.parameters.len() >= 3 {
                        let x = parse_f32(&block.parameters[0], "cog x", block.line_number)?;
                        let y = parse_f32(&block.parameters[1], "cog y", block.line_number)?;
                        let z = parse_f32(&block.parameters[2], "cog z", block.line_number)?;
                        obj.cog = Some(Vec3::new(x, y, z));
                    }
                }
                "crashmode_pole" => {
                    if block.parameters.len() >= 2 {
                        let s = parse_f32(
                            &block.parameters[0],
                            "crashmode_pole stiffness",
                            block.line_number,
                        )?;
                        let f = parse_f32(
                            &block.parameters[1],
                            "crashmode_pole factor",
                            block.line_number,
                        )?;
                        obj.crashmode_pole = Some((s, f));
                    }
                }
                "traffic_lights_group" => {
                    if let Some(val) = block.parameters.first() {
                        let cycle =
                            parse_f32(val, "traffic_lights_group cycle", block.line_number)?;
                        obj.traffic_light_groups.push(ScoTrafficLightGroup {
                            cycle_time_sec: cycle,
                            programs: Vec::new(),
                        });
                    }
                }
                "traffic_light" => {
                    if let Some(name) = block.parameters.first() {
                        let prog = ScoTrafficLightProgram {
                            name: name.clone(),
                            phases: Vec::new(),
                        };
                        if let Some(grp) = obj.traffic_light_groups.last_mut() {
                            grp.programs.push(prog);
                        } else {
                            // If program declared outside group, create implicit default group
                            obj.traffic_light_groups.push(ScoTrafficLightGroup {
                                cycle_time_sec: 0.0,
                                programs: vec![prog],
                            });
                        }
                    }
                }
                "phase" => {
                    if block.parameters.len() >= 2 {
                        let state =
                            parse_u32(&block.parameters[0], "phase state", block.line_number)?;
                        let dur =
                            parse_f32(&block.parameters[1], "phase duration", block.line_number)?;
                        let phase = ScoPhase {
                            state,
                            duration_sec: dur,
                        };
                        if let Some(grp) = obj.traffic_light_groups.last_mut() {
                            if let Some(prog) = grp.programs.last_mut() {
                                prog.phases.push(phase);
                            }
                        }
                    }
                }
                "use_traffic_light" => {
                    if let Some(val) = block.parameters.first() {
                        active_traffic_light =
                            Some(parse_usize(val, "use_traffic_light", block.line_number)?);
                    }
                }
                "approachdist" => {
                    if let Some(val) = block.parameters.first() {
                        active_approach_dist =
                            Some(parse_f32(val, "approachdist", block.line_number)?);
                    }
                }
                "path" | "path_2" => {
                    if block.parameters.len() >= 11 {
                        let x = parse_f32(&block.parameters[0], "path x", block.line_number)?;
                        let y = parse_f32(&block.parameters[1], "path y", block.line_number)?;
                        let z = parse_f32(&block.parameters[2], "path z", block.line_number)?;
                        let rot = parse_f32(&block.parameters[3], "path rot", block.line_number)?;
                        let rad =
                            parse_f32(&block.parameters[4], "path radius", block.line_number)?;
                        let len =
                            parse_f32(&block.parameters[5], "path length", block.line_number)?;
                        let s0 = parse_f32(&block.parameters[6], "path slope0", block.line_number)?;
                        let s1 = parse_f32(&block.parameters[7], "path slope1", block.line_number)?;
                        let ptype =
                            parse_u32(&block.parameters[8], "path type", block.line_number)?;
                        let width =
                            parse_f32(&block.parameters[9], "path width", block.line_number)?;
                        let dir = parse_u32(&block.parameters[10], "path dir", block.line_number)?;
                        let extra = block.parameters.get(11).and_then(|s| s.parse::<u32>().ok());

                        obj.paths.push(ScoPath {
                            start: Vec3::new(x, y, z),
                            heading_deg: rot,
                            radius_m: rad,
                            length_m: len,
                            slope_start: s0,
                            slope_end: s1,
                            path_type: ptype,
                            width_m: width,
                            direction: dir,
                            extra,
                            traffic_light_index: active_traffic_light,
                            approach_distance_m: active_approach_dist,
                        });
                    }
                }
                "maplight" => {
                    if block.parameters.len() >= 7 {
                        let x = parse_f32(&block.parameters[0], "maplight x", block.line_number)?;
                        let y = parse_f32(&block.parameters[1], "maplight y", block.line_number)?;
                        let z = parse_f32(&block.parameters[2], "maplight z", block.line_number)?;
                        let r = parse_f32(&block.parameters[3], "maplight r", block.line_number)?;
                        let g = parse_f32(&block.parameters[4], "maplight g", block.line_number)?;
                        let b = parse_f32(&block.parameters[5], "maplight b", block.line_number)?;
                        let rad =
                            parse_f32(&block.parameters[6], "maplight radius", block.line_number)?;
                        obj.map_lights.push(ScoMapLight {
                            position: Vec3::new(x, y, z),
                            color: Vec3::new(r, g, b),
                            radius: rad,
                        });
                    }
                }
                "splinehelper" => {
                    if block.parameters.len() >= 5 {
                        let file = block.parameters[0].clone();
                        let x =
                            parse_f32(&block.parameters[1], "splinehelper x", block.line_number)?;
                        let y =
                            parse_f32(&block.parameters[2], "splinehelper y", block.line_number)?;
                        let z =
                            parse_f32(&block.parameters[3], "splinehelper z", block.line_number)?;
                        let rot =
                            parse_f32(&block.parameters[4], "splinehelper rot", block.line_number)?;
                        let mut extra = Vec::new();
                        for p in &block.parameters[5..] {
                            if let Ok(v) = parse_float(p) {
                                extra.push(v);
                            }
                        }
                        obj.spline_helpers.push(ScoSplineHelper {
                            profile_file: file,
                            position: Vec3::new(x, y, z),
                            rotation_deg: rot,
                            extra,
                        });
                    }
                }
                "texttexture" | "texttexture_enh" => {
                    if block.parameters.len() >= 8 {
                        // A number indexes a string variable; anything else names one.
                        let (s_idx, s_var) = match block.parameters[0].trim().parse::<usize>() {
                            Ok(i) => (i, None),
                            Err(_) => (0, Some(block.parameters[0].trim().to_owned())),
                        };
                        let font = block.parameters[1].clone();
                        let w = parse_u32(
                            &block.parameters[2],
                            "texttexture width",
                            block.line_number,
                        )?;
                        let h = parse_u32(
                            &block.parameters[3],
                            "texttexture height",
                            block.line_number,
                        )?;
                        let mono =
                            parse_u32(&block.parameters[4], "texttexture mono", block.line_number)?;
                        let r = parse_u8(&block.parameters[5], "texttexture r", block.line_number)?;
                        let g = parse_u8(&block.parameters[6], "texttexture g", block.line_number)?;
                        let b = parse_u8(&block.parameters[7], "texttexture b", block.line_number)?;
                        obj.text_textures.push(ScoTextTexture {
                            string_index: s_idx,
                            string_var: s_var,
                            font_name: font,
                            width: w,
                            height: h,
                            color_mode: mono,
                            text_color: [r, g, b],
                        });
                    }
                }
                "ctc" => {
                    if block.parameters.len() >= 3 {
                        let var = block.parameters[0].clone();
                        let dir = block.parameters[1].clone();
                        let idx =
                            parse_usize(&block.parameters[2], "CTC index", block.line_number)?;
                        obj.ctc = Some(ScoCtc {
                            var_name: var,
                            texture_dir: dir,
                            default_index: idx,
                        });
                    }
                }
                "ctctexture" => {
                    if block.parameters.len() >= 2 {
                        let name = block.parameters[0].clone();
                        let file = block.parameters[1].clone();
                        obj.ctc_textures.push((name, file));
                    }
                }
                "script" => {
                    parse_counted_list(
                        &block.parameters,
                        &mut obj.scripts,
                        "script",
                        block.line_number,
                    )?;
                }
                "varnamelist" => {
                    parse_counted_list(
                        &block.parameters,
                        &mut obj.varnamelists,
                        "varnamelist",
                        block.line_number,
                    )?;
                }
                "stringvarnamelist" => {
                    parse_counted_list(
                        &block.parameters,
                        &mut obj.stringvarnamelists,
                        "stringvarnamelist",
                        block.line_number,
                    )?;
                }
                "constfile" => {
                    parse_counted_list(
                        &block.parameters,
                        &mut obj.constfiles,
                        "constfile",
                        block.line_number,
                    )?;
                }
                "sound" => {
                    if let Some(val) = block.parameters.first() {
                        obj.sound = Some(val.clone());
                    }
                }
                "passengercabin" => {
                    if let Some(val) = block.parameters.first() {
                        obj.passenger_cabin = Some(val.clone());
                    }
                }
                _ => {
                    // Unknown or comment block - ignore gracefully
                }
            }
        }

        Ok(obj)
    }
}

fn parse_counted_list(
    params: &[String],
    target: &mut Vec<String>,
    name: &'static str,
    line: usize,
) -> Result<(), SceneryParseError> {
    if let Some(first) = params.first() {
        let count = parse_usize(first, name, line)?;
        for item in params.iter().skip(1).take(count) {
            target.push(item.clone());
        }
    }
    Ok(())
}

fn parse_anim_block(block: &OmsiBlock) -> Result<ScoAnimation, SceneryParseError> {
    let mut commands = Vec::new();
    let mut current_cmd: Option<(String, Vec<String>)> = None;

    for param in &block.parameters {
        if param.starts_with("anim_") || param.starts_with("origin_") {
            if let Some(prev) = current_cmd.take() {
                commands.push(prev);
            }
            current_cmd = Some((param.clone(), Vec::new()));
        } else if let Some(ref mut cmd) = current_cmd {
            cmd.1.push(param.clone());
        } else {
            commands.push((param.clone(), Vec::new()));
        }
    }
    if let Some(cmd) = current_cmd {
        commands.push(cmd);
    }

    Ok(ScoAnimation { commands })
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

fn parse_u8(s: &str, field: &'static str, line: usize) -> Result<u8, SceneryParseError> {
    s.trim()
        .parse::<u8>()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_sco() {
        let text = r#"
[groups]
2
Aachen
Ampeln

[friendlyname]
Ampeln_Fussgaenger

[trafficlight]

[mesh]
Fussgaenger.o3d

[shadow]

[mesh]
Fuss_Rot.o3d

[visible]
red
1

[boundingbox]
0.3
0.3
1.2
0.0
0.0
0.6
"#;
        let sco = SceneryObject::from_text(text).unwrap();
        assert_eq!(sco.groups, vec!["Aachen", "Ampeln"]);
        assert_eq!(sco.friendly_name.as_deref(), Some("Ampeln_Fussgaenger"));
        assert!(sco.is_traffic_light);
        assert_eq!(sco.meshes.len(), 2);
        assert_eq!(sco.meshes[0].file, "Fussgaenger.o3d");
        assert_eq!(sco.meshes[0].cast_shadow, Some(true));
        assert_eq!(sco.meshes[1].file, "Fuss_Rot.o3d");
        assert_eq!(sco.meshes[1].visible, Some(("red".to_string(), 1.0)));

        let bbox = sco.bounding_box.unwrap();
        assert_eq!(bbox.half_extents, Vec3::new(0.3, 0.3, 1.2));
        assert_eq!(bbox.center, Vec3::new(0.0, 0.0, 0.6));
    }

    #[test]
    fn test_parse_traffic_light_and_paths() {
        let text = r#"
[traffic_lights_group]
99.99

[traffic_light]
Main
[phase]
6
43.0
[phase]
10
3.0
[phase]
0
52.0

[use_traffic_light]
0

[approachdist]
25.0

[path]
10.0
20.0
0.5
90.0
-999.0
35.0
0.0
0.0
0
3.5
0
"#;
        let sco = SceneryObject::from_text(text).unwrap();
        assert_eq!(sco.traffic_light_groups.len(), 1);
        let grp = &sco.traffic_light_groups[0];
        assert_eq!(grp.cycle_time_sec, 99.99);
        assert_eq!(grp.programs.len(), 1);
        assert_eq!(grp.programs[0].name, "Main");
        assert_eq!(grp.programs[0].phases.len(), 3);
        assert_eq!(grp.programs[0].phases[0].state, 6);
        assert_eq!(grp.programs[0].phases[0].duration_sec, 43.0);

        assert_eq!(sco.paths.len(), 1);
        let path = &sco.paths[0];
        assert_eq!(path.start, Vec3::new(10.0, 20.0, 0.5));
        assert_eq!(path.heading_deg, 90.0);
        assert_eq!(path.radius_m, -999.0);
        assert_eq!(path.length_m, 35.0);
        assert_eq!(path.path_type, 0);
        assert_eq!(path.width_m, 3.5);
        assert_eq!(path.direction, 0);
        assert_eq!(path.traffic_light_index, Some(0));
        assert_eq!(path.approach_distance_m, Some(25.0));
    }

    #[test]
    fn parses_tree_helper() {
        let sco =
            SceneryObject::parse(b"[tree]\nTree_Large_04.tga\n20\n30\n0.7\n1.0\n[onlyeditor]\n")
                .expect("parse");
        let tree = sco.tree.expect("tree");
        assert_eq!(tree.texture, "Tree_Large_04.tga");
        assert_eq!((tree.min_height, tree.max_height), (20.0, 30.0));
        assert_eq!((tree.min_width_ratio, tree.max_width_ratio), (0.7, 1.0));
        assert!(sco.only_editor);
    }
}
