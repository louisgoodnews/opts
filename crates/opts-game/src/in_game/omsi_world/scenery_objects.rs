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

mod finish;
mod signal;
mod template;

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
    // `[matl_nightmap]` is the material's `night_map`, drawn as it is (it is the lit picture,
    // not a mask that lights the texture like a light map).
    let texture = m.light_map.clone()?;
    let variable = m
        .light_map_variable
        .clone()
        .unwrap_or_else(|| opts_bevy::scenery_runtime::NIGHT_VARIABLE.to_owned());
    Some(opts_core::formats::vehicles::model::LightMap { texture, variable })
}

fn problems(diags: &[Diagnostic]) -> usize {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Error || d.message.contains("not found"))
        .count()
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
    opts_bevy::season::set_snow_cover(opts_bevy::season::snows(next.weather.precip_type));
    if *clock != next {
        *clock = next;
    }
}
