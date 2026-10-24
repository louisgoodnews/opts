//! Runtime of placed scenery objects: `[LOD]` selection, `[visible]` and `[newanim]` rules,
//! scenery scripts (one engine per object), light effects and light maps, and the pool of point
//! lights behind the `[maplight]` ground lights.
//!
//! Design and the benchmark interface: `docs/src/architecture/scenery-runtime.md`. The map loader
//! builds each object's hierarchy with [`build_scenery_object`]; [`SceneryRuntimePlugin`] ticks
//! the objects in `Update` (scripts by distance band and budget, LOD and light sprites every
//! frame). Hosts that drive time themselves use [`SceneryClock`], [`SceneryVariableOverrides`]
//! and [`advance_scenery`].

mod alpha;
mod clock;
mod draw_distance;
mod effects;
mod lights;
mod lod;
mod maplights;
mod scripts;
mod spawn;
mod tick;

use bevy::prelude::*;

pub use alpha::{bind_alpha_scale, scenery_alpha_scale_system, SceneryAlphaScale};
pub use clock::{solar_altitude_deg, HostVar, SceneryClock, VarRef, NIGHT_SUN_ALT_DEG};
pub use draw_distance::{
    is_culled, scenery_draw_distance_system, DistanceCulled, SceneryDrawDistance, SceneryDrawRoot,
};
pub use effects::{
    read_var, EffectTargets, SceneryLight, SceneryLightMap, SceneryMeshRule, SceneryObjectRuntime,
};
pub use lights::{scenery_light_system, SceneryLightAssets, LIGHT_DRAW_DISTANCE_M};
pub use lod::{
    scenery_lod_system, screen_size, select_band, SceneryLod, SceneryLodBias, SceneryLodMode,
};
pub use maplights::{
    assign as assign_map_lights, choose_nearest, map_light_lumens, map_light_pool_system,
    map_light_range, spawn_map_lights, MapLight, MapLightPool, MapLightSettings, PoolSlot,
};
pub use scripts::{HostValues, SceneryScripts, SceneryVariableOverrides};
pub use spawn::{
    build_scenery_object, needs_runtime, SceneryBuildAssets, SceneryObjectInput, NIGHT_VARIABLE,
};
pub use tick::{
    advance_scenery, scenery_script_system, SceneryRuntimeSettings, SceneryRuntimeStats, MAX_STEP_S,
};

/// System set of the scenery runtime; hosts update [`SceneryClock`] before it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneryRuntimeSet;

/// Registers the scenery runtime resources and systems.
#[derive(Default)]
pub struct SceneryRuntimePlugin;

impl Plugin for SceneryRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneryClock>()
            .init_resource::<SceneryRuntimeSettings>()
            .init_resource::<SceneryRuntimeStats>()
            .init_resource::<SceneryVariableOverrides>()
            .init_resource::<SceneryLodMode>()
            .init_resource::<SceneryLodBias>()
            .init_resource::<SceneryDrawDistance>()
            .init_resource::<MapLightSettings>()
            .init_resource::<MapLightPool>()
            .add_systems(
                Update,
                (
                    scenery_script_system,
                    scenery_lod_system,
                    scenery_draw_distance_system,
                    scenery_light_system,
                    scenery_alpha_scale_system,
                    map_light_pool_system,
                )
                    .chain()
                    .in_set(SceneryRuntimeSet)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}
