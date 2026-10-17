//! Map-side wiring of the scenery runtime (`docs/src/architecture/scenery-runtime.md`): compiled
//! script sets per `.sco`, `[varparent]` signal links, and the hand-over of a placed object's
//! mesh parts to `opts_bevy::scenery_runtime`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use opts_bevy::scenery_runtime::{
    build_scenery_object, needs_runtime, SceneryBuildAssets, SceneryClock, SceneryLightAssets,
    SceneryObjectInput, SceneryScripts,
};
use opts_bevy::{attach_model_parts, ModelPart};
use opts_core::formats::map::{MapObjectInstance, MapTile};
use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::{load_scenery_scripts, Diagnostic, ScriptEngine, Severity};
use opts_traffic::SignalProgram;

use super::WorldLoadJob;

/// A `.sco`'s script set, compiled once and forked per placed object.
pub struct ScriptTemplate {
    /// The compiled engine (a mutex because the load job is a shared resource).
    engine: Mutex<ScriptEngine>,
    /// String variable names in `[stringvarnamelist]` order.
    pub string_names: Vec<String>,
    /// Load diagnostics (missing files, compile problems).
    diagnostics: Vec<Diagnostic>,
    /// The `.sco` has scripts (not only string variable lists).
    has_scripts: bool,
}

/// Counters of the scenery runtime wiring.
#[derive(Debug, Clone, Default)]
pub struct SceneryWiringStats {
    /// Objects placed with a script engine.
    pub scripted_objects: usize,
    /// Distinct `.sco` script sets compiled.
    pub script_sets: usize,
    /// Of those, sets with compile errors or missing files.
    pub script_sets_with_problems: usize,
    /// Signal heads linked to their junction's program.
    pub signals_linked: usize,
    /// Signal heads whose junction or program was not found.
    pub signals_unlinked: usize,
}

/// Script templates, light assets and the object index for `[varparent]` links.
#[derive(Default)]
pub struct SceneryObjectCache {
    /// `OPTS_DEV_LOOK_AT` developer aid.
    pub dev_look: super::dev_look::DevLookTarget,
    templates: HashMap<PathBuf, Option<Arc<ScriptTemplate>>>,
    lights: SceneryLightAssets,
    /// `id_code` → `.sco` of every object on the tiles loaded so far.
    objects: HashMap<u64, String>,
    /// `[varparent]` links of the tile being loaded: child id → (parent id, program index).
    links: HashMap<u64, (u64, usize)>,
    /// Counters.
    pub stats: SceneryWiringStats,
}

impl SceneryObjectCache {
    /// An empty cache; reads the `OPTS_DEV_LOOK_AT` developer aid.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dev_look: super::dev_look::DevLookTarget::from_env(),
            ..Self::default()
        }
    }

    /// Forgets every compiled script.
    pub fn clear_templates(&mut self) {
        self.templates.clear();
    }

    /// Records the objects and `[varparent]` links of a tile about to be spawned.
    pub fn index_tile(&mut self, tile: &MapTile) {
        for o in &tile.objects {
            self.objects.insert(o.id_code, o.sco_file.clone());
        }
        self.links.clear();
        for &(index, parent) in &tile.var_parents {
            let Some(child) = tile.objects.get(index) else {
                continue;
            };
            // The head's only string variable is the program index (scenery-runtime.md).
            let program = match child.string_vars.as_slice() {
                [s] => s.trim().parse().unwrap_or(0),
                _ => 0,
            };
            self.links.insert(child.id_code, (parent, program));
        }
    }
}

/// The light map of a `.sco` material for the model loader: its own variable, or the night
/// switch when it names none; a `[matl_nightmap]` without a light map is one switched by night.
#[must_use]
pub fn light_map_of(
    m: &opts_core::formats::scenery::ScoMaterial,
) -> Option<opts_core::formats::vehicles::model::LightMap> {
    let night = || opts_bevy::scenery_runtime::NIGHT_VARIABLE.to_owned();
    if let Some(texture) = m.light_map.clone() {
        let variable = m.light_map_variable.clone().unwrap_or_else(night);
        return Some(opts_core::formats::vehicles::model::LightMap { texture, variable });
    }
    // `[matl_nightmap]`: the night texture (lit windows) is drawn as emissive light switched by
    // the night switch, like a variable-less light map (OMSI swaps the texture at night ❓).
    let texture = m.night_map.clone().filter(|t| !t.trim().is_empty())?;
    Some(opts_core::formats::vehicles::model::LightMap {
        texture,
        variable: night(),
    })
}

fn problems(diags: &[Diagnostic]) -> usize {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Error || d.message.contains("not found"))
        .count()
}

impl WorldLoadJob {
    /// The compiled script set of `sco` (`None` when it has no scripts or string lists).
    pub(super) fn script_template(
        &mut self,
        sco_path: &Path,
        sco: &SceneryObject,
    ) -> Option<Arc<ScriptTemplate>> {
        if sco.scripts.is_empty() && sco.stringvarnamelists.is_empty() {
            return None;
        }
        if let Some(t) = self.scenery.templates.get(sco_path) {
            return t.clone();
        }
        let dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let set = load_scenery_scripts(sco, |rel| self.install.find_in(&dir, rel));
        let bad = problems(&set.diagnostics);
        self.scenery.stats.script_sets += 1;
        if bad > 0 {
            self.scenery.stats.script_sets_with_problems += 1;
            tracing::warn!(sco = %sco_path.display(), problems = bad, first = ?set.diagnostics.first().map(|d| &d.message), "scenery script set has problems");
        }
        let template = Arc::new(ScriptTemplate {
            engine: Mutex::new(set.engine),
            string_names: set.string_names,
            diagnostics: set.diagnostics,
            has_scripts: !sco.scripts.is_empty(),
        });
        self.scenery
            .templates
            .insert(sco_path.to_path_buf(), Some(template.clone()));
        Some(template)
    }

    /// The light program behind `TrafficLightPhase` of `obj`: its `[varparent]` junction's
    /// program, else its own first program.
    fn signal_for(
        &mut self,
        obj: &MapObjectInstance,
        sco: &SceneryObject,
    ) -> Option<SignalProgram> {
        let program_of = |s: &SceneryObject, i: usize| {
            let (g, p) = s.traffic_light(i)?;
            let phases = p
                .phases
                .iter()
                .map(|ph| (ph.state, f64::from(ph.duration_sec)))
                .collect();
            SignalProgram::in_period(
                phases,
                f64::from(g.period_sec()),
                f64::from(p.approach_distance_m.unwrap_or(0.0)),
            )
        };
        let Some(&(parent, index)) = self.scenery.links.get(&obj.id_code) else {
            return program_of(sco, 0);
        };
        let linked = self
            .scenery
            .objects
            .get(&parent)
            .cloned()
            .and_then(|parent_sco| self.sco(&parent_sco))
            .and_then(|(_, s)| program_of(&s, index));
        if linked.is_some() {
            self.scenery.stats.signals_linked += 1;
        } else {
            self.scenery.stats.signals_unlinked += 1;
            tracing::debug!(
                object = obj.id_code,
                parent,
                index,
                "signal head: junction program not found"
            );
        }
        linked
    }

    /// Whether an object is flat static scenery: no script, light, animation, visibility rule,
    /// LOD group or driven light map, so it is only ever drawn at its placement (`needs_runtime`
    /// would be false and nothing changes its parts).
    pub(super) fn is_flat_static(
        sco: &SceneryObject,
        mesh_parts: &[Vec<ModelPart>],
        template: Option<&Arc<ScriptTemplate>>,
    ) -> bool {
        template.is_none_or(|t| !t.has_scripts)
            && sco.lights.is_empty()
            && sco.meshes.iter().all(|m| {
                m.lod_threshold.is_none() && m.visible.is_none() && m.animations.is_empty()
            })
            && mesh_parts
                .iter()
                .flatten()
                .all(|p| p.light_variable.is_none())
    }

    /// Hangs a placed object's parts under `root`: flat as before, or as a runtime hierarchy
    /// when the object has LOD groups, rules, lights, light maps or scripts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_object(
        &mut self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        root: Entity,
        sco: &SceneryObject,
        obj: &MapObjectInstance,
        mesh_parts: Vec<Vec<ModelPart>>,
        template: Option<Arc<ScriptTemplate>>,
    ) {
        self.scenery.dev_look.offer(&obj.sco_file, root);
        // `[nomaplighting]`: recorded on the root (version 1 has no behaviour yet).
        if sco.no_map_lighting {
            commands.entity(root).insert(opts_bevy::NoMapLighting);
        }
        // `[maplight]` ground lights: markers the map light pool hands its point lights to.
        opts_bevy::scenery_runtime::spawn_map_lights(commands, root, sco);
        let scripts = template.filter(|t| t.has_scripts).and_then(|t| {
            let engine = t.engine.lock().ok()?.fork();
            Some(SceneryScripts::new(
                engine,
                &t.string_names,
                &obj.string_vars,
                t.diagnostics.clone(),
            ))
        });
        let mut input = SceneryObjectInput {
            sco,
            mesh_parts,
            scripts,
            signal: None,
        };
        if !needs_runtime(&input) {
            let parts: Vec<ModelPart> = input.mesh_parts.into_iter().flatten().collect();
            let spawned = attach_model_parts(commands, root, &parts, "Mesh");
            opts_bevy::bind_alpha_scale(commands, &spawned, &parts);
            return;
        }
        if input.scripts.is_some() {
            self.scenery.stats.scripted_objects += 1;
        }
        input.signal = self.signal_for(obj, sco);
        let mut assets = SceneryBuildAssets {
            meshes,
            materials,
            lights: &mut self.scenery.lights,
        };
        build_scenery_object(commands, root, input, &mut assets);
    }
}

/// Copies the session environment (date, time, sun, night, temperature) into the scenery
/// runtime's [`SceneryClock`] (unless a host drives it: `follow_session == false`). Light
/// programs use the same clock as the AI traffic.
pub fn sync_scenery_clock(
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut clock: ResMut<SceneryClock>,
) {
    if !clock.follow_session {
        return;
    }
    let Some(env) = env else {
        return;
    };
    let date = env.date();
    let mut next = SceneryClock::at(date.year, date.month, date.day, env.seconds_of_day());
    next.follow_session = true;
    next.sun_alt_deg = env.sun_alt_deg();
    next.night = env.night();
    next.temperature_c = env.temperature_c();
    next.weather = env.conditions().vars();
    if *clock != next {
        *clock = next;
    }
}
