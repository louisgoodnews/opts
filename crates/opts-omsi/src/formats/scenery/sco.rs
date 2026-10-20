//! Parser and data structures for OMSI Scenery Object (`.sco`) files.
//!
//! A `.sco` file defines a scenery object (building, tree, street lamp, junction,
//! traffic light, etc.) placed on map tiles. It specifies 3D mesh attachments,
//! materials, collision geometry, traffic paths, lights, and scripting hooks.
//!
//! See specification in `docs/src/formats/text/sco.md`.

use glam::Vec3;

use super::error::SceneryParseError;
use super::light::{parse_light_enh, parse_light_enh_2, ScoLight};
use crate::formats::text::{decode_bytes, parse_float, scan_blocks, OmsiBlock};
use crate::formats::vehicles::model::{parse_newanim, ModelAnim};

mod meshes;
mod object;
mod texts;
mod traffic;

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
    /// Variable that switches the light map (second line of `[matl_lightmap]`); `None` when the
    /// map names none (then it follows the night lighting).
    pub light_map_variable: Option<String>,
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
    /// Text texture (index into `text_textures`) drawn on this material (`[useTextTexture]`
    /// directly after the `[matl]`).
    pub text_texture: Option<usize>,
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
            light_map_variable: None,
            night_map: None,
            trans_map: false,
            no_z_write: false,
            no_z_check: false,
            free_tex: None,
            matl_change: None,
            alpha_scale: None,
            text_texture: None,
        }
    }
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
    /// Animation definitions attached to this mesh (`[newanim]`, same layout as in `model.cfg`).
    pub animations: Vec<ModelAnim>,
    /// `[onlyeditor]` after this `[mesh]`: the mesh is an editor helper and is not drawn in the
    /// game (e.g. `Trees_MC`'s `treehelper.x` next to the `[tree]` billboard, the second
    /// `Eis.o3d` of `Eisplatte.sco`). The rest of the object is unaffected.
    pub only_editor: bool,
}

impl SceneryObject {
    /// Whether the object's z in the map is an absolute elevation: objects with `[absheight]`, and
    /// crossings (objects with `[splinehelper]` arms), which are always stored at the road's height
    /// (232 of 233 unflagged crossings on Rheinhausen sit at the terrain height, not at 0).
    #[must_use]
    pub fn has_absolute_height(&self) -> bool {
        self.abs_height || !self.spline_helpers.is_empty()
    }

    /// Whether mesh `i` is drawn in the game: neither the object (`[onlyeditor]` before any
    /// `[mesh]`) nor the mesh itself (`[onlyeditor]` after it) is editor-only.
    #[must_use]
    pub fn mesh_in_game(&self, i: usize) -> bool {
        !self.only_editor && self.meshes.get(i).is_some_and(|m| !m.only_editor)
    }

    /// Whether the object has meshes and none of them is drawn in the game.
    #[must_use]
    pub fn is_editor_only(&self) -> bool {
        self.only_editor || (!self.meshes.is_empty() && self.meshes.iter().all(|m| m.only_editor))
    }

    /// The light program `index` of `[use_traffic_light]`: programs are numbered in declaration
    /// order across all groups, with the group that holds the program.
    #[must_use]
    pub fn traffic_light(
        &self,
        index: usize,
    ) -> Option<(&ScoTrafficLightGroup, &ScoTrafficLightProgram)> {
        self.traffic_light_groups
            .iter()
            .flat_map(|g| g.programs.iter().map(move |p| (g, p)))
            .nth(index)
    }
}

impl ScoTrafficLightGroup {
    /// The period every program of the group runs on, in seconds: the group's cycle time, or the
    /// longest program when one is longer (so no phase is cut off). All programs of one group
    /// share it; that shared clock is what keeps e.g. pedestrian and car greens apart.
    #[must_use]
    pub fn period_sec(&self) -> f32 {
        self.programs
            .iter()
            .map(|p| {
                p.phases
                    .iter()
                    .map(|ph| ph.duration_sec.max(0.0))
                    .sum::<f32>()
            })
            .fold(self.cycle_time_sec.max(0.0), f32::max)
    }
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
            only_editor: false,
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
    /// Sequential phase timing entries (a repeating cycle).
    pub phases: Vec<ScoPhase>,
    /// `[approachdist]` of the program: how far before the stop line vehicles check the light
    /// (`0` = none; corpus: `0`, `25`, `30`, `50`, `100`). It follows the program's phases in the
    /// file, not a path.
    pub approach_distance_m: Option<f32>,
}

impl ScoTrafficLightProgram {
    /// Length of one cycle in seconds (sum of the phase durations).
    #[must_use]
    pub fn cycle_sec(&self) -> f32 {
        self.phases.iter().map(|p| p.duration_sec).sum()
    }
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
    /// Alignment (`[texttexture_enh]` line 9): 0 centred, 1 left, 2 right; plain `[texttexture]`
    /// is centred.
    pub orientation: u32,
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

/// An attachment point (`[new_attachment]`): where objects placed with `[attachObj]` sit on this
/// one, numbered in file order (`docs/src/formats/text/sco.md`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScoAttachment {
    /// `attach_trans`: position in the object's frame (OMSI axes: x right, y forward, z up).
    pub translation: Vec3,
    /// `attach_rot_x`, `attach_rot_y`, `attach_rot_z` in degrees (z: about the up axis).
    pub rotation_deg: Vec3,
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
    /// `[onlyeditor]` before any `[mesh]`: no mesh of the object is drawn in the game (markers
    /// such as `bus_stop.sco`, `entrypoint_bus.sco`, `car_park.sco`). Non-mesh features
    /// (`[maplight]`, paths, the `[carpark_p]` replacement) still apply. An `[onlyeditor]` after
    /// a `[mesh]` hides only that mesh ([`ScoMesh::only_editor`]); see
    /// `docs/src/formats/text/sco.md`.
    pub only_editor: bool,
    /// Parking-bay marker (`[carpark_p]`): the game places a random parked car from the map's
    /// `parklist_p.txt` here (the marker itself is editor-only).
    pub carpark_p: bool,
    /// Bus stop marker (`[busstop]`): the object is a bus stop; its position and heading are
    /// where the bus stops (the generic `bus_stop.sco` is such a marker, hidden in the game).
    pub bus_stop: bool,
    /// Vehicle-style model config (`[model]`, e.g. `model\model_parked.cfg`) of a parked/static
    /// vehicle object.
    pub model_cfg: Option<String>,
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
    /// Base mesh of `[crossing_heightdeformation]` (relative to the `model/` folder): its
    /// surface bends the object's meshes, paths and spline helpers vertically
    /// ([`super::crossing::BaseSurface`]).
    pub crossing_deformation: Option<String>,
    /// Bounding box.
    pub bounding_box: Option<ScoBoundingBox>,
    /// Center of gravity.
    pub cog: Option<Vec3>,
    /// Breakable pole physics parameters: (stiffness, factor).
    pub crashmode_pole: Option<(f32, f32)>,

    /// Visual 3D mesh attachments.
    pub meshes: Vec<ScoMesh>,
    /// Attachment points for `[attachObj]` children, in file order.
    pub attachments: Vec<ScoAttachment>,
    /// Traffic path lane segments.
    pub paths: Vec<ScoPath>,
    /// Spline helper profiles.
    pub spline_helpers: Vec<ScoSplineHelper>,
    /// Map light sources.
    pub map_lights: Vec<ScoMapLight>,
    /// Light effects (`[light_enh]`, `[light_enh_2]`) in declaration order.
    pub lights: Vec<ScoLight>,
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
    /// Problems that did not stop parsing (malformed `[newanim]` or light blocks, which are
    /// skipped).
    pub warnings: Vec<String>,
}

/// What the parser carries from one block to the next.
#[derive(Default)]
struct ParseState {
    /// The distance of the last `[LOD]` block; the next `[mesh]` takes it.
    current_lod: Option<f32>,
    /// `[use_traffic_light]` governs the next `[path]` only (every controlled path of the
    /// installed junctions has its own, repeated line; the uncontrolled approach stubs between
    /// them would otherwise inherit it).
    active_traffic_light: Option<usize>,
    /// `[approachdist]` of the light that governs the next `[path]`.
    active_approach_dist: Option<f32>,
    /// Whether a `[path]` was seen yet.
    seen_path: bool,
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
        let mut st = ParseState::default();

        for block in &blocks {
            let _handled = object::apply(&mut obj, &mut st, block)?
                || meshes::apply(&mut obj, &mut st, block)?
                || traffic::apply(&mut obj, &mut st, block)?
                || texts::apply(&mut obj, block)?;
        }

        // A path's approach distance comes from the light that governs it.
        let approach: Vec<Option<f32>> = obj
            .paths
            .iter()
            .map(|p| {
                p.traffic_light_index
                    .and_then(|i| obj.traffic_light(i))
                    .and_then(|(_, prog)| prog.approach_distance_m)
            })
            .collect();
        for (path, a) in obj.paths.iter_mut().zip(approach) {
            if a.is_some() {
                path.approach_distance_m = a;
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

/// The variable of a `[matl_lightmap]`: its second line when that is a name (not a number, a file
/// or a comment).
fn light_map_variable(params: &[String]) -> Option<String> {
    let v = params.get(1)?.trim();
    let looks_like_name = v
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && !v.contains(' ')
        && !v.contains('.');
    looks_like_name.then(|| v.to_owned())
}

/// Parses a light effect block and records which mesh and LOD group it follows.
fn add_light(obj: &mut SceneryObject, block: &OmsiBlock, lod: Option<f32>) {
    let parsed = if block.keyword == "light_enh" {
        parse_light_enh(block)
    } else {
        parse_light_enh_2(block)
    };
    match parsed {
        Ok(mut light) => {
            light.after_mesh = obj.meshes.len().checked_sub(1);
            light.lod_threshold = lod;
            obj.lights.push(light);
        }
        Err(e) => obj.warnings.push(e),
    }
}

/// The sub-keys of a `[new_attachment]` block: `attach_trans x y z` and `attach_rot_x|y|z angle`
/// (8,000 `attach_trans` and 1,700 `attach_rot_z` in the installed corpus). Unknown sub-keys and
/// unreadable numbers leave the value at zero: the point still exists, so the numbering of the
/// following points stays right.
fn parse_attachment(params: &[String]) -> ScoAttachment {
    let mut a = ScoAttachment::default();
    let num = |i: usize| {
        params
            .get(i)
            .and_then(|s| parse_float(s).ok())
            .unwrap_or(0.0)
    };
    let mut i = 0;
    while i < params.len() {
        match params[i].trim().to_ascii_lowercase().as_str() {
            "attach_trans" => {
                a.translation = Vec3::new(num(i + 1), num(i + 2), num(i + 3));
                i += 4;
            }
            "attach_rot_x" => {
                a.rotation_deg.x = num(i + 1);
                i += 2;
            }
            "attach_rot_y" => {
                a.rotation_deg.y = num(i + 1);
                i += 2;
            }
            "attach_rot_z" => {
                a.rotation_deg.z = num(i + 1);
                i += 2;
            }
            _ => i += 1,
        }
    }
    a
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
mod tests;
