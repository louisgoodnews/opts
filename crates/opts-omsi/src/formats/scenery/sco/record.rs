//! The scenery object itself: the record that holds every part of one `.sco` file.

use super::*;

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
    /// Number of positions of the rail points the object is (`[switch]`); 0 for other objects.
    pub switch_positions: u32,
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
