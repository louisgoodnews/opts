//! The door cameras' pictures on the HUD (M-0109): one panel per open door at the top right,
//! below the pause button, each with the picture of `opts_bevy::door_camera` and the door's name.
//!
//! The first door that opened is at the top, the next ones follow beneath; a closed door's panel
//! fades out (the fade itself is the slot's, `opts_bevy::door_camera`) and the panels below glide
//! up. The panels exist only while the setting `camera.door_overlay` is on; they step aside for
//! the viewpoint editor, which owns that edge.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::door_camera::{DoorCamera, DoorPoints, DoorSlot, Zone};
use opts_bevy::PlayerVehicle;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::keys;
use super::layout::HudLayout;
use super::parts::{panel, texts};
use crate::in_game::instruments::{InstrumentFade, InstrumentInk};
use crate::in_game::types::InGameEntity;
use crate::settings::GameSettings;

/// Key of the setting that turns the door camera on.
pub const SETTING: &str = "camera.door_overlay";

/// The panel of a door; `y` is where it is now (layout pixels from the top), gliding to its place.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlay {
    /// The door (index into the vehicle's doors).
    pub door: usize,
    /// Current distance from the top.
    pub y: f32,
}

/// The node that shows the picture of a door.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlayImage(pub usize);

/// The door's name below the picture.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorOverlayCaption(pub usize);

/// Gap between two panels.
const GAP: f32 = 8.0;
/// Panel height besides the picture: padding, border, caption line and the row gap.
const CHROME: f32 = 44.0;
/// How fast a panel glides to its place (1/s).
const GLIDE: f32 = 8.0;

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
            cam.open.clear();
        }
    }
}

fn spawn(
    commands: &mut Commands,
    palette: &InstrumentPalette,
    image: Handle<Image>,
    width: f32,
    door: usize,
    y: f32,
) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(y),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            DoorOverlay { door, y },
            GlobalZIndex(55),
            InGameEntity,
            UiId::new(format!("hud.door_camera.{}", door + 1)),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(width),
                    aspect_ratio: Some(16.0 / 9.0),
                    ..default()
                },
                ImageNode::new(image),
                DoorOverlayImage(door),
                UiId::new(format!("hud.door_camera.{}.image", door + 1)),
            ));
            p.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                DoorOverlayCaption(door),
                UiId::new(format!("hud.door_camera.{}.caption", door + 1)),
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

type Roots<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut DoorOverlay,
        &'static mut Node,
        &'static mut InstrumentFade,
    ),
>;

/// The resources [`update_overlay`] reads.
#[derive(SystemParam)]
pub struct OverlayEnv<'w> {
    /// Real time.
    pub time: Res<'w, Time<Real>>,
    /// The door cameras' slots.
    pub cam: Res<'w, DoorCamera>,
    /// The HUD layout.
    pub layout: Res<'w, HudLayout>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The theme, once there.
    pub theme: Option<Res<'w, MaterialTheme>>,
    /// The instrument variant.
    pub variant: Res<'w, InstrumentVariant>,
    /// The viewpoint editor's state.
    pub editor: Option<Res<'w, opts_bevy::ViewpointState>>,
}

type Images<'w, 's> = Query<
    'w,
    's,
    (
        &'static DoorOverlayImage,
        &'static mut Node,
        &'static mut ImageNode,
    ),
    Without<DoorOverlay>,
>;
type Captions<'w, 's> = Query<
    'w,
    's,
    (
        &'static DoorOverlayCaption,
        &'static mut Text,
        &'static mut InstrumentFade,
    ),
    Without<DoorOverlay>,
>;

/// Every picture takes the common width and the fade of its slot.
fn fit_images(images: &mut Images, slots: &[DoorSlot], width: f32) {
    for (marker, mut node, mut image) in images {
        let w = Val::Px(width);
        if node.width != w {
            node.width = w;
        }
        if let Some(slot) = slots.iter().find(|s| s.door == marker.0) {
            let color = image.color.with_alpha(slot.fade);
            if image.color != color {
                image.color = color;
            }
        }
    }
}

/// Every caption names its door and fades with its slot.
fn fit_captions(
    captions: &mut Captions,
    slots: &[DoorSlot],
    doors: Option<&DoorPoints>,
    loc: &Localization,
) {
    for (marker, mut text, mut fade) in captions {
        let Some(slot) = slots.iter().find(|s| s.door == marker.0) else {
            continue;
        };
        let words = doors.map_or_else(String::new, |d| caption(loc, d, marker.0));
        if text.0 != words {
            text.0 = words;
        }
        if fade.0 != slot.fade {
            fade.0 = slot.fade;
        }
    }
}

/// Keeps one panel per slot: made when a door's picture appears, stacked in slot order, gliding
/// up when one above it goes, faded with the slot, removed with it.
pub fn update_overlay(
    mut commands: Commands,
    env: OverlayEnv,
    vehicles: Query<&DoorPoints, With<PlayerVehicle>>,
    mut roots: Roots,
    mut images: Images,
    mut captions: Captions,
) {
    let OverlayEnv {
        time,
        cam,
        layout,
        loc,
        theme,
        variant,
        editor,
    } = env;
    let hidden = editor.is_some_and(|e| e.editing);
    let slots: &[_] = if hidden { &[] } else { &cam.slots };
    // Panels of doors that have no slot any more.
    for (entity, overlay, _, _) in &roots {
        if !slots.iter().any(|s| s.door == overlay.door) {
            commands.entity(entity).try_despawn();
        }
    }
    // Three or four pictures are smaller so that they stay above the minimap.
    let crowd = if slots.len() > 2 { 0.68 } else { 1.0 };
    let width = if layout.compact { WIDTH.1 } else { WIDTH.0 } * crowd;
    let step = width * 9.0 / 16.0 + CHROME + GAP;
    let k = 1.0 - (-GLIDE * time.delta_secs()).exp();
    for (i, slot) in slots.iter().enumerate() {
        let target = TOP + i as f32 * step;
        let Some((_, mut overlay, mut node, mut fade)) =
            roots.iter_mut().find(|(_, o, _, _)| o.door == slot.door)
        else {
            let palette = *theme
                .as_ref()
                .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                    t.instrument(*variant)
                });
            spawn(
                &mut commands,
                &palette,
                slot.image.clone(),
                width,
                slot.door,
                target,
            );
            continue;
        };
        overlay.y = if (overlay.y - target).abs() < 0.5 {
            target
        } else {
            overlay.y + (target - overlay.y) * k
        };
        let top = Val::Px(overlay.y);
        if node.top != top {
            node.top = top;
        }
        if fade.0 != slot.fade {
            fade.0 = slot.fade;
        }
    }
    fit_images(&mut images, slots, width);
    fit_captions(&mut captions, slots, vehicles.iter().next(), &loc);
}
