//! Builds the HUD of the player's vehicle from its profile.
//!
//! The layout is fixed: the status bar at the top left, the pause button at the top right, the
//! camera label below the top bar, the alert line and the instrument cluster at the bottom centre
//! and the key hints at the bottom left. Nothing is in the middle of the view. Every colour comes
//! from the instrument tokens, no HUD entity can take focus, and the alert slot has a fixed
//! height, so nothing ever shifts.

use bevy::prelude::*;
use opts_gui::interaction::focus::PointerOnly;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::fade::{
    FadeKind, HudFade, CAMERA_LABEL_FADE, CAMERA_LABEL_SECONDS, HINTS_FADE, HINTS_HOLD,
    STATUS_FADE, STATUS_HOLD,
};
use super::keys;
use super::layout::{HudLayout, CAMERA_LABEL_MIN_TOP};
use super::minimap::{MiniMapCaption, MiniMapSurface};
use super::profile::{HudProfile, HudProfiles};
use super::values::kind_of;
use super::widget::{Row, Slot, VehicleKind};
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::{InGameEntity, InGameHudRoot, PauseMenuBtn};

use super::parts::{build_instrument, panel, panel_node, text_widget, texts, CameraLabelRow};
pub use super::parts::{
    ActiveProfile, AlertPill, ClusterBox, HudRoot, HudRow, HudSlot, HudText, HudWidgetNode, MapBox,
    Part, ReadoutBox,
};

/// Spawns the whole HUD for `profile`.
pub fn spawn_hud(commands: &mut Commands, profile: &HudProfile, palette: &InstrumentPalette) {
    let in_slot = |slot| profile.in_slot(slot);
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            GlobalZIndex(50),
            InGameEntity,
            InGameHudRoot,
            HudRoot,
            UiId::new("hud"),
        ))
        .id();

    // Top: the status bar (fades) and the pause button.
    let top = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|bar| {
            let status = in_slot(Slot::StatusBar);
            if !status.is_empty() {
                bar.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                            FlexDirection::Column,
                            4.0,
                        ),
                        RadiusLevel::Medium,
                        palette,
                    ),
                    HudSlot(Slot::StatusBar),
                    FadeKind::Status,
                    HudFade::shown(STATUS_HOLD, STATUS_FADE),
                    UiId::new("hud.slot.status-bar"),
                ))
                .with_children(|pill| {
                    for (_, w) in &status {
                        text_widget(pill, w, TypeScale::BodyMedium, InstrumentInk::Text, palette);
                    }
                });
            } else {
                bar.spawn(Node::default());
            }
            // The pause button is chrome, not a widget: pointer only, never focused.
            bar.spawn((
                (
                    Button,
                    panel_node(
                        UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                        FlexDirection::Row,
                        0.0,
                    ),
                    BorderColor::all(palette.panel_border),
                    BackgroundColor(palette.panel),
                ),
                M3Button::default(),
                PauseMenuBtn,
                PointerOnly,
                InstrumentPanel,
                RadiusToken(RadiusLevel::Medium),
                UiId::new("hud.pause"),
            ))
            .with_children(|btn| {
                btn.spawn((
                    texts(
                        keys::PAUSE_BUTTON,
                        TypeScale::Label,
                        InstrumentInk::Text,
                        palette,
                    ),
                    opts_gui::l10n::L10nKey(keys::PAUSE_BUTTON),
                ));
            });
        })
        .id();

    // The camera label: below the top bar, centred, hidden until the camera changes.
    let camera = in_slot(Slot::CameraLabel);
    let camera_label = (!camera.is_empty()).then(|| {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(CAMERA_LABEL_MIN_TOP),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                CameraLabelRow,
            ))
            .with_children(|row| {
                row.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            FlexDirection::Row,
                            0.0,
                        ),
                        RadiusLevel::Full,
                        palette,
                    ),
                    HudSlot(Slot::CameraLabel),
                    FadeKind::CameraLabel,
                    HudFade::hidden(CAMERA_LABEL_SECONDS - CAMERA_LABEL_FADE, CAMERA_LABEL_FADE),
                    Visibility::Hidden,
                    UiId::new("hud.slot.camera-label"),
                ))
                .with_children(|pill| {
                    for (_, w) in &camera {
                        text_widget(pill, w, TypeScale::BodyMedium, InstrumentInk::Text, palette);
                    }
                });
            })
            .id()
    });

    // Bottom: the cluster at the left (hints, alerts, instruments in a fixed order, top to
    // bottom) and the mini map at the right.
    let layout = HudLayout::default();
    let bottom = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexEnd,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|bottom| {
            spawn_cluster(bottom, profile, palette);
            spawn_minimap(bottom, profile, palette, &layout);
        })
        .id();

    commands.entity(root).add_children(&[top, bottom]);
    if let Some(camera_label) = camera_label {
        commands.entity(root).add_child(camera_label);
    }
}

/// The bottom-left cluster. Rows, top to bottom: the hints (fade), the alert line (a minimum
/// height: an alert coming and going moves nothing below it) and the instruments.
fn spawn_cluster(
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
        });
}

/// The mini map slot, bottom right: a panel with the map texture and a caption.
fn spawn_minimap(
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

/// Builds the HUD once the player's vehicle is known: picks the profile of its kind and spawns it.
pub fn queue_spawn(commands: &mut Commands, theme: &MaterialTheme) {
    let palette = *theme.instrument(InstrumentVariant::Day);
    commands.queue(move |world: &mut World| {
        let kind = {
            let mut q = world.query_filtered::<(
                Has<crate::in_game::omsi_world::RailPlayer>,
                Has<opts_bevy::ArticulatedChain>,
            ), With<opts_bevy::PlayerVehicle>>();
            q.iter(world)
                .next()
                .map_or(VehicleKind::Bus, |(rail, chain)| kind_of(rail, chain))
        };
        // Without the HUD plugin (tests) the shipped profiles apply.
        let profile = world.get_resource::<HudProfiles>().map_or_else(
            || HudProfiles::load(None).get(kind).cloned(),
            |p| p.get(kind).cloned(),
        );
        let Some(profile) = profile else {
            tracing::warn!(kind = kind.id(), "no HUD profile, the HUD stays empty");
            return;
        };
        tracing::info!(
            kind = kind.id(),
            widgets = profile.widgets.len(),
            "building the HUD"
        );
        let mut commands = world.commands();
        spawn_hud(&mut commands, &profile, &palette);
        world.insert_resource(ActiveProfile { kind, profile });
        world.flush();
    });
}
