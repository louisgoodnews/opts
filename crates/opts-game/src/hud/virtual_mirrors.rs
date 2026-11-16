//! The virtual mirrors on the HUD: the areas of the left mirror and the two right mirrors that
//! Tab makes clickable in the driver's view, and the panels that stream the mirror cameras
//! (`opts_bevy::mirrors`, `docs/src/architecture/virtual-mirrors.md`).
//!
//! A mirror that is switched on keeps its panel while the vehicle is driven; the areas of the
//! others show only while the mode is open. The picture is mirrored left to right, so that
//! something that comes up on the left appears on the left of the panel, as in a mirror.

use bevy::prelude::*;
use opts_bevy::camera::CameraMode;
use opts_bevy::mirrors::{MirrorSlot, VirtualMirrors};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::interaction::focus::PointerOnly;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::keys;
use super::parts::texts;

mod slot;
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::InGameEntity;
use slot::spawn_slot;

/// The area or panel of one mirror.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorSlotNode(pub MirrorSlot);

/// Width of an area or panel, in percent of the window width.
const WIDTH_VW: f32 = 15.0;
/// Edge distance, in percent of the window.
const EDGE_VW: f32 = 1.5;

/// Where the slot sits: left or right edge and the distance from the top (percent of the height).
const fn anchor(slot: MirrorSlot) -> (bool, f32) {
    match slot {
        MirrorSlot::Left => (true, 30.0),
        MirrorSlot::RightUpper => (false, 14.0),
        MirrorSlot::RightLower => (false, 46.0),
    }
}

const fn caption_key(slot: MirrorSlot) -> &'static str {
    match slot {
        MirrorSlot::Left => keys::VMIRROR_LEFT,
        MirrorSlot::RightUpper => keys::VMIRROR_RIGHT_UPPER,
        MirrorSlot::RightLower => keys::VMIRROR_RIGHT_LOWER,
    }
}

/// What the HUD shows of the virtual mirrors; the nodes are rebuilt when it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// Per slot: not shown, an area to click, or a streaming panel (with the picture's id).
    slots: [Option<Option<AssetId<Image>>>; 3],
}

/// What each slot shows given the state: `None` nothing, `Some(None)` the area, `Some(Some(id))`
/// the stream.
#[must_use]
pub fn shown(vm: &VirtualMirrors, visible: bool, external: bool) -> Shown {
    let mut slots = [None, None, None];
    if visible || external {
        for slot in MirrorSlot::ALL {
            let i = slot.index();
            let Some(feed) = &vm.feeds[i] else { continue };
            slots[i] = if vm.streams[i] {
                Some(Some(feed.picture.id()))
            } else if vm.open && visible {
                Some(None)
            } else {
                None
            };
        }
    }
    Shown { slots }
}

/// Keeps the areas and panels in step with the state of the virtual mirrors.
pub fn sync_virtual_mirrors(
    mut commands: Commands,
    vm: Res<VirtualMirrors>,
    view: super::params::MirrorView,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    nodes: Query<Entity, With<MirrorSlotNode>>,
    mut last: Local<Option<Shown>>,
) {
    let super::params::MirrorView {
        camera,
        avail,
        editor,
    } = view;
    let visible =
        camera.mode == CameraMode::Cockpit && avail.cockpit && !editor.is_some_and(|e| e.editing);
    // Streaming panels also show in the external view (the areas to click exist only in the
    // driver's view).
    let external = camera.mode == CameraMode::External;
    let now = shown(&vm, visible, external);
    if last.as_ref() == Some(&now) {
        return;
    }
    for e in &nodes {
        commands.entity(e).despawn();
    }
    let palette = theme.instrument(*variant);
    for slot in MirrorSlot::ALL {
        let Some(state) = now.slots[slot.index()] else {
            continue;
        };
        spawn_slot(&mut commands, &vm, (slot, state.is_some()), palette);
    }
    *last = Some(now);
}

/// A click on an area or panel switches that mirror, while the mode is open.
pub fn mirror_clicks(
    mut clicks: MessageReader<ButtonClicked>,
    nodes: Query<&MirrorSlotNode>,
    mut vm: ResMut<VirtualMirrors>,
) {
    for ButtonClicked(entity) in clicks.read() {
        let Ok(MirrorSlotNode(slot)) = nodes.get(*entity) else {
            continue;
        };
        if vm.open {
            vm.toggle(*slot);
        }
    }
}
