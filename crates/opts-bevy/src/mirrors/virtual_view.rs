//! Virtual mirrors: the pictures of the mirror cameras as panels on the screen.
//!
//! In the driver's view Tab opens the mode; the three areas of the left mirror and the two right
//! mirrors can then be clicked (`opts_game::hud::virtual_mirrors`). A mirror that is switched on
//! streams its camera (the one that sits at the mirror, see [`super::MirrorCamera`]) into a panel
//! at full rate and at twice the picture size, whether or not the real pane is in view. The mode
//! only decides whether the areas can be clicked; the panels stay while the vehicle is driven.
//!
//! * **Slots.** [`assign_slots`] names the mirrors by where they sit on the vehicle: the highest
//!   side mirror on the left is [`MirrorSlot::Left`], the two highest on the right are
//!   [`MirrorSlot::RightUpper`] and [`MirrorSlot::RightLower`] (main mirror and wide-angle
//!   mirror). The interior mirror and cameras along the body get no slot; variants that are
//!   hidden never do.
//! * **Mouse.** A mouse that steers (`MouseDriving`) is let go while the mode is open, so that a
//!   click is a click.

use super::*;
use crate::camera::{CameraAvailability, CameraMode, CameraState, ViewpointState};
use crate::mouse_driving::MouseDriving;

/// Number of mirror areas.
pub const SLOT_COUNT: usize = 3;

/// A mirror of the player's vehicle, named by its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorSlot {
    /// The (highest) mirror on the left side.
    Left,
    /// The highest mirror on the right side.
    RightUpper,
    /// The second highest mirror on the right side (the wide-angle mirror).
    RightLower,
}

impl MirrorSlot {
    /// All slots in index order.
    pub const ALL: [Self; SLOT_COUNT] = [Self::Left, Self::RightUpper, Self::RightLower];

    /// Index into [`VirtualMirrors::streams`] and [`VirtualMirrors::feeds`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::RightUpper => 1,
            Self::RightLower => 2,
        }
    }
}

/// The camera and picture behind one slot.
#[derive(Debug, Clone)]
pub struct MirrorFeed {
    /// The mirror camera.
    pub camera: Entity,
    /// The picture it renders into (the panel shows this handle).
    pub picture: Handle<Image>,
    /// Width over height of the picture.
    pub aspect: f32,
}

/// State of the virtual mirrors.
#[derive(Resource, Debug, Clone, Default)]
pub struct VirtualMirrors {
    /// The areas can be clicked (Tab).
    pub open: bool,
    /// Which mirrors stream into a panel.
    pub streams: [bool; SLOT_COUNT],
    /// The mirror behind each slot, as the mirror system found it this frame.
    pub feeds: [Option<MirrorFeed>; SLOT_COUNT],
    /// The mouse was let go by opening the mode and is given back on closing it.
    pub released_mouse: bool,
}

impl VirtualMirrors {
    /// Whether any mirror is switched on.
    #[must_use]
    pub fn any_stream(&self) -> bool {
        self.streams.iter().any(|s| *s)
    }

    /// Whether the player's mirrors need their cameras for the virtual mirrors: a mirror streams,
    /// or the areas are open to be clicked. Without this the areas need cameras that only exist
    /// when the light quality's `mirror_count` is above 0, so with mirrors off (the automatic
    /// quality drops them at level 4) Tab showed nothing to click.
    #[must_use]
    pub fn needs_cameras(&self) -> bool {
        self.open || self.any_stream()
    }

    /// The camera of `slot` when its stream is on and the vehicle has such a mirror.
    #[must_use]
    pub fn streaming_camera(&self, slot: MirrorSlot) -> Option<Entity> {
        let i = slot.index();
        self.streams[i]
            .then(|| self.feeds[i].as_ref().map(|f| f.camera))
            .flatten()
    }

    /// Switches the stream of `slot`.
    pub fn toggle(&mut self, slot: MirrorSlot) {
        let s = &mut self.streams[slot.index()];
        *s = !*s;
    }
}

/// Mirrors closer to the centre line than this (m) are not side mirrors (the interior mirror).
pub const SIDE_MIRROR_MIN_X_M: f32 = 0.8;
/// A side mirror lies within this distance (m) of the foremost one on its side; mirrors further
/// back (cameras along the body) are not the driver's.
pub const SIDE_MIRROR_FRONT_BAND_M: f32 = 1.5;

/// Names the mirror groups by place. `groups` holds each group's camera and the centre of its
/// visible panes in the vehicle's frame (Bevy axes: x right, y up, z backwards).
///
/// Only side mirrors count: at least [`SIDE_MIRROR_MIN_X_M`] from the centre line and in the front
/// [`SIDE_MIRROR_FRONT_BAND_M`] of their side. Of those, the highest is the upper one.
#[must_use]
pub fn assign_slots(groups: &[(Entity, Vec3)]) -> [Option<Entity>; SLOT_COUNT] {
    let side = |right: bool| -> Vec<(Entity, Vec3)> {
        let on_side: Vec<(Entity, Vec3)> = groups
            .iter()
            .copied()
            .filter(|g| g.1.x.abs() >= SIDE_MIRROR_MIN_X_M && (g.1.x >= 0.0) == right)
            .collect();
        let foremost = on_side
            .iter()
            .map(|g| -g.1.z)
            .fold(f32::NEG_INFINITY, f32::max);
        let mut front: Vec<(Entity, Vec3)> = on_side
            .into_iter()
            .filter(|g| -g.1.z >= foremost - SIDE_MIRROR_FRONT_BAND_M)
            .collect();
        front.sort_by(|a, b| b.1.y.total_cmp(&a.1.y).then(a.0.cmp(&b.0)));
        front
    };
    let (left, right) = (side(false), side(true));
    [
        left.first().map(|g| g.0),
        right.first().map(|g| g.0),
        right.get(1).map(|g| g.0),
    ]
}

/// The size of a picture while it streams: twice its normal size, at most 1024 a side.
#[must_use]
pub fn stream_size(base: UVec2) -> UVec2 {
    (base * 2).min(UVec2::splat(1024))
}

/// Parameters for [`toggle_virtual_mirrors`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct ToggleVirtualMirrorsParams<'w> {
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    pub camera: Res<'w, CameraState>,
    pub avail: Res<'w, CameraAvailability>,
    pub editor: Option<Res<'w, ViewpointState>>,
    pub mouse: Option<ResMut<'w, MouseDriving>>,
    pub state: ResMut<'w, VirtualMirrors>,
}

/// Tab (without Shift) opens and closes the mode in the driver's view; leaving that view closes
/// it. Shift + Tab is the viewpoint editor's, which keeps Tab while it is open.
pub fn toggle_virtual_mirrors(mut params: ToggleVirtualMirrorsParams) {
    let in_cockpit = params.camera.mode == CameraMode::Cockpit && params.avail.cockpit;
    let editing = params.editor.as_ref().is_some_and(|e| e.editing);
    let shift = params.keys.pressed(KeyCode::ShiftLeft) || params.keys.pressed(KeyCode::ShiftRight);
    let want = if !in_cockpit || editing {
        false
    } else if params.keys.just_pressed(KeyCode::Tab) && !shift {
        !params.state.open
    } else {
        params.state.open
    };
    if want == params.state.open {
        return;
    }
    params.state.open = want;
    if let Some(mut mouse) = params.mouse {
        if want && mouse.drives() {
            mouse.freed = true;
            params.state.released_mouse = true;
        } else if !want && params.state.released_mouse {
            mouse.freed = false;
            params.state.released_mouse = false;
        }
    }
}
