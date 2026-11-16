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
    ActiveProfile, AlertPill, ClusterBox, HudBarFill, HudBarTrack, HudRoot, HudRow, HudSlot,
    HudText, HudWidgetNode, MapBox, Part, ReadoutBox,
};

mod camera_label;
mod top_bar;
use camera_label::spawn_camera_label;
use top_bar::spawn_top_bar;

/// Spawns the whole HUD for `profile`.
pub fn spawn_hud(commands: &mut Commands, profile: &HudProfile, palette: &InstrumentPalette) {
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

    let top = spawn_top_bar(commands, profile, palette);

    let camera_label = spawn_camera_label(commands, profile, palette);

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

mod cluster;
use cluster::{spawn_cluster, spawn_minimap};

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
