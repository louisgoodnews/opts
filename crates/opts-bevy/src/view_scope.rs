//! Meshes that belong to one side of the hull: `[viewpoint]` 1 (outside) and 2 (inside).
//!
//! A vehicle model lists some meshes twice, once for the view from outside and once for the view
//! from inside (the Hamburg buses: `17_aussenspiegel.o3d`, flag 5, and `17_aussensp_innen.o3d`,
//! flag 2, both are the exterior mirror). Drawing both puts two mirrors side by side. The loader
//! tags such meshes with a [`ViewScope`]; [`apply_view_scope`] hides the ones that do not belong to
//! the camera's side. See `docs/src/formats/text/vehicles/model_cfg.md`, section *Viewpoint Flags*.
//!
//! The camera is **inside** in the cockpit and passenger modes and **outside** otherwise (external,
//! free roam and the on-foot ego camera). A mesh with both bits (3) or none (0) is never tagged.

use bevy::prelude::*;
use opts_core::formats::vehicles::model::ViewpointFlags;

use crate::camera::{CameraMode, CameraState};

/// The side of the hull a mesh is drawn from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewScope {
    /// Drawn when the camera is outside.
    pub outside: bool,
    /// Drawn when the camera is inside.
    pub inside: bool,
    /// Whether [`apply_view_scope`] hid the mesh (so it knows to show it again).
    hidden_by_scope: bool,
}

impl ViewScope {
    /// The scope of a mesh with `flags`, `None` when it is drawn from both sides.
    #[must_use]
    pub fn of(flags: ViewpointFlags) -> Option<Self> {
        let raw = flags.raw();
        let (outside, inside) = (raw & 1 != 0, raw & 2 != 0);
        (outside != inside).then_some(Self {
            outside,
            inside,
            hidden_by_scope: false,
        })
    }

    /// Whether the mesh is drawn for a camera that is `inside` or outside the hull.
    #[must_use]
    pub const fn shown(&self, camera_inside: bool) -> bool {
        if camera_inside {
            self.inside
        } else {
            self.outside
        }
    }
}

/// Whether a camera mode looks from inside the vehicle.
#[must_use]
pub const fn camera_is_inside(mode: CameraMode) -> bool {
    matches!(mode, CameraMode::Cockpit | CameraMode::Passenger)
}

/// Hides the meshes whose [`ViewScope`] excludes the camera's side and shows them again afterwards.
/// Runs in `PostUpdate`, after the systems that also write `Visibility` (a `[visible]` condition).
pub fn apply_view_scope(
    camera: Option<Res<CameraState>>,
    mut meshes: Query<(&mut ViewScope, &mut Visibility)>,
) {
    let inside = camera.is_some_and(|c| camera_is_inside(c.mode));
    for (mut scope, mut visibility) in &mut meshes {
        if scope.shown(inside) {
            if scope.hidden_by_scope {
                scope.hidden_by_scope = false;
                *visibility = Visibility::Inherited;
            }
        } else {
            scope.hidden_by_scope = true;
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// Registers [`apply_view_scope`].
#[derive(Default)]
pub struct ViewScopePlugin;

impl Plugin for ViewScopePlugin {
    fn build(&self, app: &mut App) {
        // After every `Update` writer of `Visibility` (animation `[visible]`, lights, alpha scale),
        // before the visibility is propagated to the hierarchy.
        app.add_systems(
            PostUpdate,
            apply_view_scope
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(raw: u32) -> ViewpointFlags {
        ViewpointFlags::new(raw)
    }

    #[test]
    fn only_single_sided_meshes_are_scoped() {
        assert!(ViewScope::of(flags(0)).is_none(), "always");
        assert!(ViewScope::of(flags(3)).is_none(), "both sides");
        assert!(
            ViewScope::of(flags(4)).is_none(),
            "AI only is dropped by the loader"
        );
        assert_eq!(
            ViewScope::of(flags(1)).map(|s| (s.outside, s.inside)),
            Some((true, false))
        );
        assert_eq!(
            ViewScope::of(flags(2)).map(|s| (s.outside, s.inside)),
            Some((false, true))
        );
        // The exterior mirror of the Hamburg buses: outside plus AI.
        assert_eq!(
            ViewScope::of(flags(5)).map(|s| (s.outside, s.inside)),
            Some((true, false))
        );
        assert_eq!(
            ViewScope::of(flags(6)).map(|s| (s.outside, s.inside)),
            Some((false, true))
        );
        assert!(ViewScope::of(flags(7)).is_none());
    }

    #[test]
    fn cockpit_and_passenger_look_from_inside() {
        assert!(camera_is_inside(CameraMode::Cockpit));
        assert!(camera_is_inside(CameraMode::Passenger));
        for m in [CameraMode::External, CameraMode::FreeRoam, CameraMode::Ego] {
            assert!(!camera_is_inside(m));
        }
    }

    fn app_with(mode: CameraMode) -> App {
        let mut app = App::new();
        app.insert_resource(CameraState {
            mode,
            ..CameraState::default()
        });
        app.add_systems(Update, apply_view_scope);
        app
    }

    #[test]
    fn the_twin_mirror_meshes_never_show_together() {
        let mut app = app_with(CameraMode::External);
        let outer = app
            .world_mut()
            .spawn((ViewScope::of(flags(5)).unwrap(), Visibility::Inherited))
            .id();
        let inner = app
            .world_mut()
            .spawn((ViewScope::of(flags(2)).unwrap(), Visibility::Inherited))
            .id();
        let both = app.world_mut().spawn(Visibility::Inherited).id();
        let vis = |app: &App, e| *app.world().get::<Visibility>(e).unwrap();
        app.update();
        assert_eq!(vis(&app, outer), Visibility::Inherited);
        assert_eq!(vis(&app, inner), Visibility::Hidden);
        app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
        app.update();
        assert_eq!(vis(&app, outer), Visibility::Hidden);
        assert_eq!(vis(&app, inner), Visibility::Inherited);
        app.world_mut().resource_mut::<CameraState>().mode = CameraMode::FreeRoam;
        app.update();
        assert_eq!(vis(&app, outer), Visibility::Inherited);
        assert_eq!(vis(&app, inner), Visibility::Hidden);
        assert_eq!(vis(&app, both), Visibility::Inherited);
    }
}
