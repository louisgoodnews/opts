//! The viewpoint editor panel of the driver's view (M-0084), in the HUD's look.
//!
//! The panel exists only while the editor is open (Tab in view 2). It sits at the right edge, clear
//! of the middle of the view, is painted from the instrument tokens, takes **no focus** (buttons
//! are `PointerOnly`; the keyboard belongs to the editor while it is open, see
//! `opts_bevy::camera::viewpoint`) and shows every change live because the camera reads the same
//! state. One row per value: a name, a minus button, a track (click or drag to set), a plus button
//! and the number in centimetres or degrees.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use opts_bevy::camera::viewpoint::{ViewpointRow, ViewpointState};
use opts_bevy::{BusVehicle, PlayerVehicle};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::interaction::focus::PointerOnly;
use opts_gui::interaction::state::Pressed;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use super::keys;
use super::parts::{panel, panel_node, texts};
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::InGameEntity;

mod buttons;
mod panel;
mod rows;

use buttons::{action_buttons, small_button};
pub use panel::spawn_panel;
use rows::value_row;

/// The root of the panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointPanel;

/// What a button does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewpointButton {
    /// One step down or up on a row.
    Step(ViewpointRow, i8),
    /// Back to the vehicle's own view.
    Reset,
    /// Close and return to the stored view.
    Cancel,
    /// Store and close.
    Save,
    /// Switch between saving for this vehicle and for all of its model.
    Scope,
}

/// The text of the scope button (this vehicle / all of this model).
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointScopeText;

/// The scope button's row, hidden when the vehicle's model is unknown.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointScopeRow;

/// The track of a row (click or drag sets the value).
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointTrack(pub ViewpointRow);

/// The filled part of a track.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointFill(pub ViewpointRow);

/// The number of a row.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointValue(pub ViewpointRow);

/// The marker in front of the selected row (a shape, so colour is not the only signal).
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointMarker(pub ViewpointRow);

/// The name of a row.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointName(pub ViewpointRow);

/// The "view n/m" line.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointSubtitle;

/// The "not saved yet" line.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointUnsaved;

/// Colours of tracks and fills, painted from the instrument palette like the panels.
#[derive(Component, Debug, Clone, Copy)]
pub struct ViewpointPaint(pub bool);

const ROW_NAMES: [(ViewpointRow, &str); 4] = [
    (ViewpointRow::X, keys::VIEWPOINT_X),
    (ViewpointRow::Y, keys::VIEWPOINT_Y),
    (ViewpointRow::Z, keys::VIEWPOINT_Z),
    (ViewpointRow::Fov, keys::VIEWPOINT_FOV),
];

mod params;
mod systems;
pub use systems::{panel_buttons, panel_tracks, sync_panel, update_panel, value_text};
