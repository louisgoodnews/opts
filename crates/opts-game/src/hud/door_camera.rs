//! The door camera's picture on the HUD (M-0109): a panel at the top right, below the pause
//! button, with the picture of `opts_bevy::door_camera` and the door's name.
//!
//! It exists only while a door is shown (the setting `camera.door_overlay` is on and F4 chose a
//! door or a door leaf moves); it steps aside for the viewpoint editor, which owns that edge.

use bevy::prelude::*;
use opts_bevy::door_camera::{DoorCamera, DoorPoints, Zone};
use opts_bevy::PlayerVehicle;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::keys;
use super::layout::HudLayout;
use super::parts::{panel, texts};
use crate::in_game::instruments::InstrumentInk;
use crate::in_game::types::InGameEntity;
use crate::settings::GameSettings;

/// Key of the setting that turns the door camera on.
pub const SETTING: &str = "camera.door_overlay";

/// The panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlay;

/// The node that shows the picture.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlayImage;

/// The door's name below the picture.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlayCaption;

/// Width of the picture in layout pixels, in the normal and the compact layout.
const WIDTH: (f32, f32) = (320.0, 224.0);
/// Distance from the top (below the pause button).
const TOP: f32 = 72.0;

/// Copies the setting into the door camera.
pub fn sync_setting(settings: Res<GameSettings>, mut cam: ResMut<DoorCamera>) {
    if !settings.is_changed() && !cam.is_added() {
        return;
    }
    let on = settings
        .raw(SETTING)
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
    if cam.enabled != on {
        cam.enabled = on;
        if !on {
            cam.manual = None;
            cam.auto = None;
        }
    }
}

fn spawn(commands: &mut Commands, palette: &InstrumentPalette, image: Handle<Image>, width: f32) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(TOP),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            DoorOverlay,
            GlobalZIndex(55),
            InGameEntity,
            UiId::new("hud.door_camera"),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(width),
                    aspect_ratio: Some(16.0 / 9.0),
                    ..default()
                },
                ImageNode::new(image),
                DoorOverlayImage,
                UiId::new("hud.door_camera.image"),
            ));
            p.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                DoorOverlayCaption,
                UiId::new("hud.door_camera.caption"),
            ));
        });
}

/// The words of a door's name: its number, kind, place along the vehicle and side.
fn caption(loc: &Localization, doors: &DoorPoints, door: usize) -> String {
    let Some(d) = doors.0.get(door) else {
        return String::new();
    };
    let zone = match doors.zone(door) {
        Zone::Front => keys::DOOR_FRONT,
        Zone::Middle => keys::DOOR_MIDDLE,
        Zone::Rear => keys::DOOR_REAR,
    };
    loc.t_args(
        keys::DOOR_TITLE,
        &[
            ("number".to_owned(), Arg::from(door as i32 + 1)),
            (
                "kind".to_owned(),
                Arg::from(loc.t(if d.entry {
                    keys::DOOR_ENTRY
                } else {
                    keys::DOOR_EXIT
                })),
            ),
            ("zone".to_owned(), Arg::from(loc.t(zone))),
            (
                "side".to_owned(),
                Arg::from(loc.t(if d.side < 0 {
                    keys::DOOR_LEFT
                } else {
                    keys::DOOR_RIGHT
                })),
            ),
        ],
    )
}

/// Builds the panel when a door is shown and removes it when none is; keeps the door's name.
#[allow(clippy::too_many_arguments)]
pub fn update_overlay(
    mut commands: Commands,
    cam: Res<DoorCamera>,
    layout: Res<HudLayout>,
    loc: Res<Localization>,
    theme: Option<Res<MaterialTheme>>,
    variant: Res<InstrumentVariant>,
    editor: Option<Res<opts_bevy::ViewpointState>>,
    vehicles: Query<&DoorPoints, With<PlayerVehicle>>,
    roots: Query<(Entity, &Children), With<DoorOverlay>>,
    mut images: Query<&mut Node, With<DoorOverlayImage>>,
    mut captions: Query<&mut Text, With<DoorOverlayCaption>>,
) {
    let want = cam
        .shown()
        .filter(|_| !editor.is_some_and(|e| e.editing) && cam.image.is_some());
    let existing = roots.iter().next().map(|(e, _)| e);
    let (Some(door), Some(image)) = (want, cam.image.clone()) else {
        if let Some(root) = existing {
            commands.entity(root).try_despawn();
        }
        return;
    };
    let width = if layout.compact { WIDTH.1 } else { WIDTH.0 };
    if existing.is_none() {
        let palette = *theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*variant)
            });
        spawn(&mut commands, &palette, image, width);
        return;
    }
    for mut node in &mut images {
        let w = Val::Px(width);
        if node.width != w {
            node.width = w;
        }
    }
    let text = vehicles
        .iter()
        .next()
        .map_or_else(String::new, |d| caption(&loc, d, door));
    for mut t in &mut captions {
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}
