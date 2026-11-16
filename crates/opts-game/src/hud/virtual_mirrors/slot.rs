//! Spawns one virtual mirror slot node with its picture, caption and click target.

use super::*;

/// One slot of the mirrors: a clickable area, or a streaming panel with the mirror's picture
/// (`streaming`), anchored at its edge of the screen.
pub(super) fn spawn_slot(
    commands: &mut Commands,
    vm: &VirtualMirrors,
    (slot, streaming): (MirrorSlot, bool),
    palette: &InstrumentPalette,
) {
    let i = slot.index();
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
        BackgroundColor(if streaming {
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
    match (streaming, feed) {
        (true, Some(feed)) => {
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
