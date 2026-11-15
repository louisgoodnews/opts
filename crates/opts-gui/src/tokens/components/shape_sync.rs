//! Shape token systems: the corner radius and the drop shadow of an elevation level.

use bevy::prelude::*;

use crate::theme::theme::MaterialTheme;

use super::{ElevationToken, RadiusToken};

/// Query item alias for elevation shadow synchronization.
pub type ElevationSyncItem<'a> = (Entity, Ref<'a, ElevationToken>, Option<&'a BoxShadow>);

/// System that gives every raised entity (an [`ElevationToken`] above level 0) its drop
/// shadow, and refreshes it when the level or the theme changes.
pub fn sync_elevation_shadows(
    theme: Res<MaterialTheme>,
    mut commands: Commands,
    query: Query<ElevationSyncItem, With<Node>>,
) {
    let theme_changed = theme.is_changed();
    for (entity, level, current) in &query {
        if !(theme_changed || level.is_changed()) && current.is_some() {
            continue;
        }
        match level.0.shadow(&theme.scheme) {
            Some(shadow) => {
                if current != Some(&shadow) {
                    commands.entity(entity).insert(shadow);
                }
            }
            None if current.is_some() => {
                commands.entity(entity).remove::<BoxShadow>();
            }
            None => {}
        }
    }
}
/// System that synchronizes the node's border radius with [`RadiusToken`].
pub fn sync_radius_tokens(mut query: Query<(Ref<RadiusToken>, &mut Node)>) {
    for (token, mut node) in &mut query {
        let radius = token.0.to_border_radius();
        if token.is_changed() && node.border_radius != radius {
            node.border_radius = radius;
        }
    }
}
