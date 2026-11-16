//! The mini map slot of the bottom-right corner: a panel with the map texture and a caption.

use super::*;

pub(in crate::hud::build) fn spawn_minimap(
    bottom: &mut ChildSpawnerCommands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
    layout: &HudLayout,
) {
    let Some((_, w)) = profile.in_slot(Slot::MiniMap).into_iter().next() else {
        bottom.spawn(Node::default());
        return;
    };
    bottom
        .spawn((
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            HudSlot(Slot::MiniMap),
            UiId::new("hud.slot.minimap"),
        ))
        .with_children(|slot| {
            slot.spawn((
                panel(
                    Node {
                        width: Val::Px(layout.map_side),
                        padding: UiRect::all(Val::Px(6.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        align_items: AlignItems::Stretch,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    RadiusLevel::Large,
                    palette,
                ),
                MapBox,
                HudWidgetNode {
                    id: w.id,
                    slot: w.slot,
                },
                UiId::new(format!("hud.{}", w.id)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        aspect_ratio: Some(1.0),
                        ..default()
                    },
                    ImageNode::default(),
                    MiniMapSurface,
                    UiId::new("hud.minimap.surface"),
                ));
                panel.spawn((
                    texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                    MiniMapCaption,
                    UiId::new("hud.minimap.caption"),
                ));
            });
        });
}
