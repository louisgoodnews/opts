//! The resources a drive starts with.

use super::*;

/// The resources of a drive.
pub(super) fn add_resources(app: &mut App) {
    app.init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
        .init_resource::<omsi_world::GroundIndex>()
        .init_resource::<opts_bevy::text_texture::RedrawnImages>()
        .init_resource::<omsi_world::ChronoState>()
        .init_resource::<omsi_world::DeveloperDetail>()
        .init_resource::<omsi_world::CollisionIndex>()
        .init_resource::<omsi_world::RailNetwork>()
        .init_resource::<omsi_world::BikeNetwork>()
        .init_resource::<omsi_world::BikeSettings>()
        .init_resource::<omsi_world::BikeAiPool>()
        .init_resource::<omsi_world::RailSettings>()
        .init_resource::<omsi_world::RailAiPool>()
        .init_resource::<omsi_world::PedestrianNetwork>()
        .init_resource::<omsi_world::PedestrianSettings>()
        .init_resource::<omsi_world::PedestrianPool>()
        .init_resource::<omsi_world::TrafficState>()
        .init_resource::<omsi_world::TrafficSettings>()
        .init_resource::<omsi_world::TrafficPool>()
        .init_resource::<omsi_world::ScheduledTraffic>()
        .init_resource::<SimulationLoadState>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<opts_bevy::CameraState>()
        .init_resource::<opts_bevy::CameraAvailability>()
        .insert_resource(omsi_world::WorldStats {
            // The developer panel is off unless the setting says otherwise.
            overlay_visible: false,
            ..default()
        });
}
