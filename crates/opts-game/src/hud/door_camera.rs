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

mod env;
mod fit;
mod spawn;

use env::{OverlayEnv, Roots};
use fit::{fit_captions, fit_images, Captions, Images};
use spawn::spawn;

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
