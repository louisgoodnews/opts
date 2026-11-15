//! Pointing the eyes: the head anchor of the driver, the player head and hiding the own character in the seat views.

use super::*;

/// Marks the player's head as the eye position once the hierarchy is propagated (see [`AlignEyes`]).
pub fn align_driver_eyes(
    mut commands: bevy::prelude::Commands,
    mut humans: bevy::prelude::Query<(Entity, &Human, &mut AlignEyes)>,
    globals: bevy::prelude::Query<&bevy::prelude::GlobalTransform>,
) {
    for (e, human, mut align) in &mut humans {
        if align.wait > 0 {
            align.wait -= 1;
            continue;
        }
        if globals.get(human.rig.head_anchor).is_err() {
            continue;
        }
        // The human stays on the seat (`[drivpos]`); the cockpit and ego cameras go to its eyes.
        commands.entity(e).remove::<AlignEyes>();
        commands.entity(human.rig.head_anchor).insert(DriverHead);
    }
}

/// Puts [`PlayerHead`] on the [`DriverHead`] whose vehicle is the active one and nowhere else.
pub fn sync_player_head(
    mut commands: bevy::prelude::Commands,
    heads: bevy::prelude::Query<
        (Entity, bevy::prelude::Has<PlayerHead>),
        bevy::prelude::With<DriverHead>,
    >,
    parents: bevy::prelude::Query<&bevy::prelude::ChildOf>,
    players: bevy::prelude::Query<(), bevy::prelude::With<crate::vehicle_sim::PlayerVehicle>>,
) {
    for (head, has) in &heads {
        let active = players.contains(parents.root_ancestor(head));
        if active && !has {
            commands.entity(head).insert(PlayerHead);
        } else if !active && has {
            commands.entity(head).remove::<PlayerHead>();
        }
    }
}

/// Hides the player's own character while the camera sits at the driver's seat (cockpit and ego
/// views), so the eye never looks through the head or the torso; shows it in every other view.
///
/// The character is the player's when it belongs to the player's vehicle (it is a child of its
/// root): that holds from the frame it exists, whereas [`PlayerHead`] is only set a few frames
/// later (eye alignment, [`sync_player_head`]), which showed the back of the head in the first
/// seconds of a drive in the driver's view.
pub fn hide_driver_in_seat_views(
    state: bevy::prelude::Res<crate::camera::CameraState>,
    humans: bevy::prelude::Query<
        (Entity, &mut bevy::prelude::Visibility),
        bevy::prelude::With<Human>,
    >,
    parents: bevy::prelude::Query<&bevy::prelude::ChildOf>,
    players: bevy::prelude::Query<(), bevy::prelude::With<crate::vehicle_sim::PlayerVehicle>>,
) {
    let seat = matches!(
        state.mode,
        crate::camera::CameraMode::Cockpit | crate::camera::CameraMode::Ego
    );
    let mut humans = humans;
    for (human, mut vis) in &mut humans {
        if !players.contains(parents.root_ancestor(human)) {
            continue;
        }
        let want = if seat {
            bevy::prelude::Visibility::Hidden
        } else {
            bevy::prelude::Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// How far the eyes (the nose bridge) lie in front of the head's centre line (m).
pub const EYES_FORWARD_M: f32 = 0.09;
