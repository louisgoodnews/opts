//! Draw distance of scenery objects (M-0029): objects whose bounding sphere lies entirely
//! beyond the limit are hidden, and shown again once they are back within it.

use bevy::prelude::*;

use super::SceneryLod;

/// Extent assumed for an object without a measured bounding sphere (m).
pub const DEFAULT_EXTENT_M: f32 = 50.0;
/// Distance by which an object must come back inside the limit before it is shown again (m).
pub const HYSTERESIS_M: f32 = 20.0;
/// Seconds between two culling passes.
const INTERVAL_S: f32 = 0.25;

/// How far scenery objects are drawn from the main camera.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct SceneryDrawDistance {
    /// Limit in metres; `None` draws every object that is resident.
    pub metres: Option<f32>,
}

/// Marks the root of a placed scenery object as subject to the draw distance (every placed
/// object, not only those with a runtime hierarchy).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SceneryDrawRoot;

/// Marks an object root hidden by the draw distance (and only those: the system never touches
/// the visibility of an object it did not hide itself).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DistanceCulled;

/// Whether an object at `distance` with bounding `extent` is culled by `limit`; `was_culled`
/// applies the hysteresis so objects at the border do not flicker.
#[must_use]
pub fn is_culled(distance: f32, extent: f32, limit: f32, was_culled: bool) -> bool {
    let inside = distance - extent;
    if was_culled {
        inside > limit - HYSTERESIS_M
    } else {
        inside > limit
    }
}

type SceneryDrawObject<'a> = (
    Entity,
    &'a GlobalTransform,
    Option<&'a SceneryLod>,
    Has<DistanceCulled>,
    &'a mut Visibility,
);

/// Hides scenery object roots beyond [`SceneryDrawDistance`] and restores them.
pub fn scenery_draw_distance_system(
    time: Res<Time>,
    limit: Res<SceneryDrawDistance>,
    mut since: Local<f32>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::MainCamera>>,
    mut objects: Query<SceneryDrawObject<'_>, With<SceneryDrawRoot>>,
    mut commands: Commands,
) {
    *since += time.delta_secs();
    if *since < INTERVAL_S && !limit.is_changed() {
        return;
    }
    *since = 0.0;
    let camera = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation());
    for (entity, transform, lod, culled, mut visibility) in &mut objects {
        let want = match (limit.metres, camera) {
            (Some(m), Some(cam)) => {
                let extent = lod.map_or(DEFAULT_EXTENT_M, |l| l.radius);
                is_culled(transform.translation().distance(cam), extent, m, culled)
            }
            (Some(_), None) => culled,
            (None, _) => false,
        };
        if want == culled {
            continue;
        }
        if want {
            *visibility = Visibility::Hidden;
            commands.entity(entity).insert(DistanceCulled);
        } else {
            *visibility = Visibility::Inherited;
            commands.entity(entity).remove::<DistanceCulled>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn border_objects_do_not_flicker() {
        assert!(!is_culled(500.0, 10.0, 500.0, false));
        assert!(is_culled(511.0, 10.0, 500.0, false));
        // Culled objects stay culled until they are well inside.
        assert!(is_culled(505.0, 10.0, 500.0, true));
        assert!(!is_culled(485.0, 10.0, 500.0, true));
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<SceneryDrawDistance>()
            .add_systems(Update, scenery_draw_distance_system);
        app.world_mut().spawn((
            Camera::default(),
            crate::MainCamera,
            GlobalTransform::default(),
        ));
        app
    }

    fn object(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                SceneryDrawRoot,
                GlobalTransform::from_xyz(x, 0.0, 0.0),
                Visibility::Inherited,
            ))
            .id()
    }

    fn pass(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
    }

    #[test]
    fn far_objects_hide_and_come_back() {
        let mut app = app();
        let near = object(&mut app, 100.0);
        let far = object(&mut app, 900.0);
        pass(&mut app);
        assert_eq!(
            app.world().get::<Visibility>(far),
            Some(&Visibility::Inherited)
        );

        app.world_mut().resource_mut::<SceneryDrawDistance>().metres = Some(400.0);
        pass(&mut app);
        assert_eq!(
            app.world().get::<Visibility>(near),
            Some(&Visibility::Inherited)
        );
        assert_eq!(
            app.world().get::<Visibility>(far),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().get::<DistanceCulled>(far).is_some());

        app.world_mut().resource_mut::<SceneryDrawDistance>().metres = None;
        pass(&mut app);
        assert_eq!(
            app.world().get::<Visibility>(far),
            Some(&Visibility::Inherited)
        );
        assert!(app.world().get::<DistanceCulled>(far).is_none());
    }

    #[test]
    fn objects_hidden_by_others_stay_hidden() {
        let mut app = app();
        let e = object(&mut app, 100.0);
        *app.world_mut().get_mut::<Visibility>(e).unwrap() = Visibility::Hidden;
        app.world_mut().resource_mut::<SceneryDrawDistance>().metres = Some(400.0);
        pass(&mut app);
        app.world_mut().resource_mut::<SceneryDrawDistance>().metres = None;
        pass(&mut app);
        assert_eq!(app.world().get::<Visibility>(e), Some(&Visibility::Hidden));
    }
}
