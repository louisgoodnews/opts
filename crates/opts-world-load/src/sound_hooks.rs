//! The sound layer as the world load sees it (ISS-064): the data the load hands to the sound
//! modules of `opts-game` (`vehicle_sound`, `player_sound`, `scenery_sound`) and the hook through
//! which it calls them. The sound modules stay in `opts-game`; this crate does not depend on them.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::AiVariables;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::sound::SoundDef;
use opts_core::formats::vehicles::BusManifest;

use crate::load_job::WorldLoadJob;

/// The definitions of an AI vehicle type that can be played, with their sound files.
#[derive(Debug, Clone, Default)]
pub struct AiSoundSet {
    /// The loops the traffic model can drive (at most 16) with their files:
    /// continuous, their conditions and variables all stated by the model.
    pub voices: Vec<(Arc<SoundDef>, PathBuf)>,
    /// One-shots the traffic model raises itself (the horn, `ev_ai_horn`).
    pub events: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions that need the vehicle's own scripts (door triggers and door variables,
    /// anything else the model does not state): played only by a vehicle that runs scripts, a
    /// scheduled bus (at most 48).
    pub scripted: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions of the configuration that were left out (a condition mode that is not
    /// understood, interior-only sounds, no file, over a cap).
    pub skipped: usize,
}

/// An event the traffic model raises for an AI vehicle (`ev_AI_Horn`): it plays the one-shots of
/// the configuration that name it.
#[derive(Component, Debug, Clone, Default)]
pub struct AiEvents(pub Vec<String>);

impl AiEvents {
    /// Raises event `name`.
    pub fn raise(&mut self, name: &str) {
        if self.0.len() < 8 {
            self.0.push(name.to_owned());
        }
    }
}

/// The calls the world load makes into the sound modules of `opts-game`. Each method forwards to
/// one function there with the same arguments and in the same order as the load used to call it
/// directly (ISS-064). The implementation is `opts-game`'s `world_sound_hooks::GameSoundHooks`.
pub trait SoundHooks: Send + Sync {
    /// `vehicle_sound::ai_sound_set`: the AI sound set of a vehicle file.
    fn ai_sound_set(
        &self,
        job: &WorldLoadJob,
        manifest: &BusManifest,
        dir: &std::path::Path,
        bus: &std::path::Path,
    ) -> Option<std::sync::Arc<AiSoundSet>>;

    /// `vehicle_sound::attach_ai`: the continuous and event voices of a vehicle type under `root`.
    fn attach_ai(
        &self,
        commands: &mut Commands,
        root: Entity,
        set: Option<&std::sync::Arc<AiSoundSet>>,
    );

    /// `vehicle_sound::attach_ai_unit`: the loops of one unit's set under `unit`.
    fn attach_ai_unit(
        &self,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        set: &std::sync::Arc<AiSoundSet>,
    );

    /// `player_sound::attach_defs`: script-driven voices of a unit; returns the number attached.
    fn attach_scripted_defs(
        &self,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        defs: &[(std::sync::Arc<SoundDef>, PathBuf)],
    ) -> usize;

    /// `player_sound::attach`: the `[sound]` voices of a player vehicle under `root`.
    fn attach_player(
        &self,
        job: &WorldLoadJob,
        commands: &mut Commands,
        root: Entity,
        manifest: &BusManifest,
        bus_dir: &std::path::Path,
    );

    /// `player_sound::attach_unit`: the `[sound]` voices of one unit of a vehicle.
    fn attach_player_unit(
        &self,
        job: &WorldLoadJob,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        manifest: &BusManifest,
        bus_dir: &std::path::Path,
    );

    /// `scenery_sound::attach`: the voices of a placed scenery object under `root`.
    fn attach_scenery(
        &self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        root: Entity,
        sco: &SceneryObject,
        sco_dir: &std::path::Path,
    );

    /// `vehicle_sound::publish_sound_variables`: writes one AI vehicle's sound variables.
    fn publish_sound_variables(
        &self,
        vars: &mut AiVariables,
        speed_ms: f32,
        accel_ms2: f32,
        brake: f32,
        wheel_radius: f32,
        wet: bool,
        heavy: bool,
    );
}

/// Hooks that do nothing. A load job starts with these and `opts-game` replaces them before the
/// job is used; code that runs a job without `opts-game` (benchmarks, tests) keeps them.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSoundHooks;

impl SoundHooks for NoSoundHooks {
    fn ai_sound_set(
        &self,
        _job: &WorldLoadJob,
        _manifest: &BusManifest,
        _dir: &std::path::Path,
        _bus: &std::path::Path,
    ) -> Option<std::sync::Arc<AiSoundSet>> {
        None
    }

    fn attach_ai(
        &self,
        _commands: &mut Commands,
        _root: Entity,
        _set: Option<&std::sync::Arc<AiSoundSet>>,
    ) {
    }

    fn attach_ai_unit(
        &self,
        _commands: &mut Commands,
        _unit: Entity,
        _owner: Option<Entity>,
        _set: &std::sync::Arc<AiSoundSet>,
    ) {
    }

    fn attach_scripted_defs(
        &self,
        _commands: &mut Commands,
        _unit: Entity,
        _owner: Option<Entity>,
        _defs: &[(std::sync::Arc<SoundDef>, PathBuf)],
    ) -> usize {
        0
    }

    fn attach_player(
        &self,
        _job: &WorldLoadJob,
        _commands: &mut Commands,
        _root: Entity,
        _manifest: &BusManifest,
        _bus_dir: &std::path::Path,
    ) {
    }

    fn attach_player_unit(
        &self,
        _job: &WorldLoadJob,
        _commands: &mut Commands,
        _unit: Entity,
        _owner: Option<Entity>,
        _manifest: &BusManifest,
        _bus_dir: &std::path::Path,
    ) {
    }

    fn attach_scenery(
        &self,
        _job: &mut WorldLoadJob,
        _commands: &mut Commands,
        _root: Entity,
        _sco: &SceneryObject,
        _sco_dir: &std::path::Path,
    ) {
    }

    fn publish_sound_variables(
        &self,
        _vars: &mut AiVariables,
        _speed_ms: f32,
        _accel_ms2: f32,
        _brake: f32,
        _wheel_radius: f32,
        _wet: bool,
        _heavy: bool,
    ) {
    }
}

impl WorldLoadJob {
    /// The sound hooks the job calls. A shared handle, so a caller can pass the job along with it.
    #[must_use]
    pub fn sound(&self) -> Arc<dyn SoundHooks> {
        self.sound_hooks.clone()
    }

    /// Installs the sound hooks the job calls. `opts-game` does this before the job is used.
    pub fn set_sound_hooks(&mut self, hooks: Arc<dyn SoundHooks>) {
        self.sound_hooks = hooks;
    }
}

/// The hooks as a Bevy resource, for the systems of the load that need them (`opts-game` inserts
/// it next to the job).
#[derive(Resource, Clone)]
pub struct SoundHooksRes(pub std::sync::Arc<dyn SoundHooks>);
