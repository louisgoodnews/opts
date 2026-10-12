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
use super::profile::{HudProfile, HudProfiles};
use super::values::kind_of;
use super::widget::{Slot, VehicleKind};
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::{InGameEntity, InGameHudRoot, PauseMenuBtn};

use super::parts::{build_instrument, panel, panel_node, text_widget, texts};
pub use super::parts::{ActiveProfile, AlertPill, HudRoot, HudSlot, HudText, HudWidgetNode, Part};

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
            .spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(84.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
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

    // Bottom: the alert line above the instrument cluster.
    let bottom = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|dash| {
            if !in_slot(Slot::Alerts).is_empty() {
                // A fixed height: an alert appearing or leaving moves nothing.
                dash.spawn(Node {
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|slot| {
                    slot.spawn((
                        panel(
                            panel_node(
                                UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                                FlexDirection::Row,
                                0.0,
                            ),
                            RadiusLevel::Full,
                            palette,
                        ),
                        HudSlot(Slot::Alerts),
                        AlertPill,
                        Visibility::Hidden,
                        UiId::new("hud.slot.alerts"),
                    ))
                    .with_children(|pill| {
                        for (_, w) in in_slot(Slot::Alerts) {
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
            let instruments = in_slot(Slot::Instruments);
            if !instruments.is_empty() {
                dash.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                            FlexDirection::Row,
                            36.0,
                        ),
                        RadiusLevel::Large,
                        palette,
                    ),
                    HudSlot(Slot::Instruments),
                    UiId::new("hud.slot.instruments"),
                ))
                .with_children(|cluster| {
                    for (_, w) in &instruments {
                        build_instrument(cluster, w, palette);
                    }
                });
            }
        })
        .id();

    // The hints: bottom left, fading.
    let hints = in_slot(Slot::Hints);
    let hint_pill = (!hints.is_empty()).then(|| {
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                max_width: Val::Percent(36.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    panel(
                        panel_node(
                            UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                            FlexDirection::Column,
                            0.0,
                        ),
                        RadiusLevel::Medium,
                        palette,
                    ),
                    HudSlot(Slot::Hints),
                    FadeKind::Hints,
                    HudFade::shown(HINTS_HOLD, HINTS_FADE),
                    UiId::new("hud.slot.hints"),
                ))
                .with_children(|pill| {
                    for (_, w) in &hints {
                        text_widget(pill, w, TypeScale::Label, InstrumentInk::Text, palette);
                    }
                });
            })
            .id()
    });

    commands.entity(root).add_children(&[top, bottom]);
    for extra in [camera_label, hint_pill].into_iter().flatten() {
        commands.entity(root).add_child(extra);
    }
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
