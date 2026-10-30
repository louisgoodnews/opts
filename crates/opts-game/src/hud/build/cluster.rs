//! Bottom-left instrument cluster and bottom-right mini-map slot builders for the HUD.

use super::*;


/// The hint pill, when the profile has hint widgets.
fn hints_pill(
    cluster: &mut ChildSpawnerCommands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) {
    let hints = profile.in_row(Row::Hints);
    if !hints.is_empty() {
        cluster
            .spawn((
                panel(
                    panel_node(
                        UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        FlexDirection::Column,
                        0.0,
                    ),
                    RadiusLevel::Medium,
                    palette,
                ),
                ClusterBox,
                HudRow(Row::Hints),
                FadeKind::Hints,
                HudFade::shown(HINTS_HOLD, HINTS_FADE),
                UiId::new("hud.slot.hints"),
            ))
            .with_children(|pill| {
                for (_, w) in &hints {
                    text_widget(pill, w, TypeScale::Label, InstrumentInk::Text, palette);
                }
            });
    }
}

/// The alert slot: a fixed-height row with the alert pill, so an alert coming or going moves nothing.
fn alerts_slot(
    cluster: &mut ChildSpawnerCommands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) {
    let alerts = profile.in_row(Row::Alerts);
    if !alerts.is_empty() {
        // A minimum height: an alert appearing or leaving moves nothing.
        cluster
            .spawn((
                Node {
                    min_height: Val::Px(44.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ClusterBox,
                HudRow(Row::Alerts),
            ))
            .with_children(|slot| {
                slot.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                            FlexDirection::Row,
                            0.0,
                        ),
                        RadiusLevel::Large,
                        palette,
                    ),
                    ClusterBox,
                    AlertPill,
                    Visibility::Hidden,
                    UiId::new("hud.slot.alerts"),
                ))
                .with_children(|pill| {
                    for (_, w) in &alerts {
                        text_widget(
                            pill,
                            w,
                            TypeScale::BodyLarge,
                            InstrumentInk::Warning,
                            palette,
                        );
                    }
                });
            });
    }
}

/// The instrument row, when the profile has instruments.
fn instruments_row(
    cluster: &mut ChildSpawnerCommands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) {
    let instruments = profile.in_row(Row::Instruments);
    if !instruments.is_empty() {
        cluster
            .spawn((
                panel(
                    panel_node(
                        UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                        FlexDirection::Row,
                        28.0,
                    ),
                    RadiusLevel::Large,
                    palette,
                ),
                HudRow(Row::Instruments),
                UiId::new("hud.slot.instruments"),
            ))
            .with_children(|row| {
                for (_, w) in &instruments {
                    build_instrument(row, w, palette);
                }
            });
    }
}

/// The bottom-left cluster. Rows, top to bottom: the hints (fade), the alert line (a minimum
/// height: an alert coming and going moves nothing below it) and the instruments.
pub(super) fn spawn_cluster(
    bottom: &mut ChildSpawnerCommands,
    profile: &HudProfile,
    palette: &InstrumentPalette,
) {
    let has_row = |row: Row| !profile.in_row(row).is_empty();
    if !Row::ALL.iter().any(|r| has_row(*r)) {
        bottom.spawn(Node::default());
        return;
    }
    bottom
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(8.0),
                flex_shrink: 1.0,
                ..default()
            },
            ClusterBox,
            HudSlot(Slot::ClusterBottomLeft),
            UiId::new("hud.slot.cluster"),
        ))
        .with_children(|cluster| {
            hints_pill(cluster, profile, palette);
            alerts_slot(cluster, profile, palette);
            instruments_row(cluster, profile, palette);
        });
}

/// The mini map slot, bottom right: a panel with the map texture and a caption.
pub(super) fn spawn_minimap(
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
