//! Types of the script-effect scenarios: the probes, captured states, diagnostics and inputs.

use super::*;

/// State of one light at a captured step.
#[derive(Debug, Clone, PartialEq)]
pub struct LightProbe {
    /// Brightness the runtime computed (0..1).
    pub level: f32,
    /// The runtime shows the sprite (not hidden by distance or cone).
    pub shown: bool,
    /// World position (Bevy axes).
    pub pos: Vec3,
    /// Sprite diameter at full brightness (m).
    pub size_m: f32,
}

/// What the runtime holds for the object at a captured step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    /// All script variables (lower-case names).
    pub vars: BTreeMap<String, f32>,
    /// Per `[mesh]`: the animation values the mesh currently shows (empty without `[newanim]`).
    pub shown: Vec<Vec<f32>>,
    /// Lights in `.sco` order.
    pub lights: Vec<LightProbe>,
    /// Applied brightness of each light map (NaN before the first update).
    pub light_maps: Vec<f32>,
}

/// One captured step.
#[derive(Debug, Clone)]
pub struct State {
    /// `s0`, `s1`, `s2`, `s3`, `s4_<phase>`, `s0b`.
    pub name: String,
    /// Both views over black and white.
    pub pixels: Vec<ViewPixels>,
    /// The runtime's state.
    pub probe: Probe,
}

/// One script diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDiag {
    /// Severity is `Error`.
    pub error: bool,
    /// Raised while running (not while loading).
    pub runtime: bool,
    /// Script file as given.
    pub file: String,
    /// Message.
    pub message: String,
}

/// Everything the scenarios produced.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Captured states in scenario order.
    pub states: Vec<State>,
    /// Distinct diagnostics.
    pub diagnostics: Vec<ScriptDiag>,
    /// The spawned object carried a script engine.
    pub engine_found: bool,
    /// Load-time diagnostics of the engine (those after this index were raised while running).
    pub load_len: Option<usize>,
    /// The runtime entities of the current placement (taken before a seeded fault detaches them).
    pub(super) handles: Handles,
}

/// Entities of the object's runtime, in `.sco` order.
#[derive(Debug, Clone, Default)]
pub(super) struct Handles {
    pub(super) meshes: Vec<Entity>,
    pub(super) lights: Vec<Entity>,
    pub(super) light_maps: Vec<Entity>,
}

/// Inputs of a run.
pub struct Ctx<'a> {
    /// The object.
    pub entry: &'a ObjectEntry,
    /// Its preflight.
    pub pre: &'a Preflight,
    /// String set A of the placement.
    pub strings: &'a [String],
    /// Where the tile places it.
    pub tile_place: mapgen::Placement,
    /// The views (framed on the expected geometry).
    pub views: &'a [View],
    /// Time budget end.
    pub deadline: Instant,
    /// Thresholds.
    pub thresholds: &'a ScriptThresholds,
    /// Seeded fault: detach the object's effects from the runtime.
    pub effects_off: bool,
}
