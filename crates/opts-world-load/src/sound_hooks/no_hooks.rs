//! The do-nothing sound hooks, for a load job that runs without `opts-game`.

use super::*;

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
