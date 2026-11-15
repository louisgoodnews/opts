//! Registers the full-size map's resources and systems.

use super::*;

/// Registers the resources and systems of the full-size map.
pub struct FullMapPlugin;

impl Plugin for FullMapPlugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        let open = |s: Res<FullMapState>| s.open;
        app.add_plugins(crate::hud::route_path::RoutePathPlugin)
            .init_resource::<FullMapState>()
            .init_resource::<FullMapCanvas>()
            .init_resource::<FullMapImage>()
            .init_resource::<OverviewRoads>()
            .add_systems(
                OnEnter(AppState::InGame),
                |mut state: ResMut<FullMapState>,
                 mut image: ResMut<FullMapImage>,
                 mut overview: ResMut<OverviewRoads>| {
                    *overview = OverviewRoads::default();
                    let span = state.remembered_span_m;
                    *state = FullMapState {
                        remembered_span_m: span,
                        ..FullMapState::default()
                    };
                    image.0 = None;
                },
            )
            .add_systems(
                Update,
                (
                    toggle,
                    input.run_if(open),
                    follow.run_if(open),
                    redraw.run_if(open),
                    overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
