//! The sound modules' side of the world load (ISS-064): `opts-world-load` calls the sound layer
//! through its `SoundHooks` trait, and this forwards each call to the function it stands for, with
//! the same arguments.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::AiVariables;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::sound::SoundDef;
use opts_core::formats::vehicles::BusManifest;
use opts_world_load::sound_hooks::{AiSoundSet, SoundHooks};

use crate::in_game::omsi_world::WorldLoadJob;

/// The sound hooks of the game: the `vehicle_sound`, `player_sound` and `scenery_sound` modules.
#[derive(Debug, Clone, Copy, Default)]
pub struct GameSoundHooks;

impl SoundHooks for GameSoundHooks {
    fn ai_sound_set(
        &self,
        job: &WorldLoadJob,
        manifest: &BusManifest,
        dir: &Path,
        bus: &Path,
    ) -> Option<Arc<AiSoundSet>> {
        crate::vehicle_sound::ai_sound_set(job, manifest, dir, bus)
    }

    fn attach_ai(&self, commands: &mut Commands, root: Entity, set: Option<&Arc<AiSoundSet>>) {
        crate::vehicle_sound::attach_ai(commands, root, set);
    }

    fn attach_ai_unit(
        &self,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        set: &Arc<AiSoundSet>,
    ) {
        crate::vehicle_sound::attach_ai_unit(commands, unit, owner, set);
    }

    fn attach_scripted_defs(
        &self,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        defs: &[(Arc<SoundDef>, PathBuf)],
    ) -> usize {
        crate::player_sound::attach_defs(commands, unit, owner, defs)
    }

    fn attach_player(
        &self,
        job: &WorldLoadJob,
        commands: &mut Commands,
        root: Entity,
        manifest: &BusManifest,
        bus_dir: &Path,
    ) {
        crate::player_sound::attach(job, commands, root, manifest, bus_dir);
    }

    fn attach_player_unit(
        &self,
        job: &WorldLoadJob,
        commands: &mut Commands,
        unit: Entity,
        owner: Option<Entity>,
        manifest: &BusManifest,
        bus_dir: &Path,
    ) {
        crate::player_sound::attach_unit(job, commands, unit, owner, manifest, bus_dir);
    }

    fn attach_scenery(
        &self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        root: Entity,
        sco: &SceneryObject,
        sco_dir: &Path,
    ) {
        crate::scenery_sound::attach(job, commands, root, sco, sco_dir);
    }

    fn publish_sound_variables(
        &self,
        vars: &mut AiVariables,
        speed_ms: f32,
        accel_ms2: f32,
        brake: f32,
        wheel_radius: f32,
        wet: bool,
        heavy: bool,
    ) {
        crate::vehicle_sound::publish_sound_variables(
            vars,
            speed_ms,
            accel_ms2,
            brake,
            wheel_radius,
            wet,
            heavy,
        );
    }
}
