//! The virtual mirrors on the HUD: the areas of the left mirror and the two right mirrors that
//! Tab makes clickable in the driver's view, and the panels that stream the mirror cameras
//! (`opts_bevy::mirrors`, `docs/src/architecture/virtual-mirrors.md`).
//!
//! A mirror that is switched on keeps its panel while the vehicle is driven; the areas of the
//! others show only while the mode is open. The picture is mirrored left to right, so that
//! something that comes up on the left appears on the left of the panel, as in a mirror.

use bevy::prelude::*;
use opts_bevy::camera::viewpoint::ViewpointState;
use opts_bevy::camera::{CameraAvailability, CameraMode, CameraState};
use opts_bevy::mirrors::{MirrorSlot, VirtualMirrors};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::interaction::focus::PointerOnly;
use opts_gui::prelude::*;
use opts_gui::theme::InstrumentVariant;

use super::keys;
use super::parts::texts;
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::InGameEntity;

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
#[allow(clippy::too_many_arguments)]
pub fn sync_virtual_mirrors(
    mut commands: Commands,
    vm: Res<VirtualMirrors>,
    camera: Res<CameraState>,
    avail: Res<CameraAvailability>,
    editor: Option<Res<ViewpointState>>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    nodes: Query<Entity, With<MirrorSlotNode>>,
    mut last: Local<Option<Shown>>,
) {
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
        let i = slot.index();
        let Some(state) = now.slots[i] else { continue };
        let feed = vm.feeds[i].as_ref();
        let (left, top) = anchor(slot);
        let aspect = feed.map_or(1.0, |f| f.aspect).clamp(0.4, 3.0);
        let mut node = Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(top),
            width: Val::Vw(WIDTH_VW),
            aspect_ratio: Some(aspect),
            border: UiRect::all(Val::Px(2.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        };
        if left {
            node.left = Val::Vw(EDGE_VW);
        } else {
            node.right = Val::Vw(EDGE_VW);
        }
        let mut entity = commands.spawn((
            Button,
            node,
            BorderColor::all(palette.panel_border),
            BackgroundColor(if state.is_some() {
                palette.panel
            } else {
                palette.panel.with_alpha(0.55)
            }),
            PointerOnly,
            InstrumentPanel,
            MirrorSlotNode(slot),
            GlobalZIndex(54),
            InGameEntity,
            UiId::new(format!("hud.virtual_mirror.{i}")),
        ));
        match (state, feed) {
            (Some(_), Some(feed)) => {
                let mut image = ImageNode::new(feed.picture.clone());
                image.flip_x = true;
                entity.with_children(|p| {
                    p.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        image,
                    ));
                });
            }
            _ => {
                entity.with_children(|p| {
                    p.spawn((
                        texts(
                            caption_key(slot),
                            TypeScale::BodyMedium,
                            InstrumentInk::Text,
                            palette,
                        ),
                        opts_gui::l10n::L10nKey(caption_key(slot)),
                    ));
                });
            }
        }
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
