//! Effects of settings that are not read by a typed consumer: texture filtering, look, language,
//! log level, the developer panel, the tiles, the scenery and the render quality.

mod apply;
mod render;
mod scenery;
mod tiles;

pub use apply::{
    apply_live_changes, apply_saved_language, apply_saved_trace, sync_developer_panel,
};
pub use render::{
    hdr_enabled, sync_audio, sync_hdr, sync_tree_wind, sync_weather_effects, weather_effects_of,
};
pub use scenery::{
    draw_distance_of, shadow_cascades, shadow_detail_of, sync_detail_distances, sync_shadow_detail,
    sync_view_distance, DETAIL_DISTANCE_DEFAULT_PERCENT, LIGHT_DISTANCE_DEFAULT_M,
    SHADOW_DETAIL_DEFAULT, SHADOW_DISTANCE_DEFAULT_M, VIEW_DISTANCE_STEP_M,
    VIEW_DISTANCE_UNLIMITED,
};
pub use tiles::{sync_tile_loading, sync_water};

#[cfg(test)]
mod tests;
