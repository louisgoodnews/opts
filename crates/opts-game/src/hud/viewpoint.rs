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

fn small_button(
    parent: &mut ChildSpawnerCommands,
    action: ViewpointButton,
    key: &'static str,
    id: String,
    palette: &InstrumentPalette,
) {
    parent
        .spawn((
            (
                Button,
                panel_node(
                    UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    FlexDirection::Row,
                    0.0,
                ),
                BorderColor::all(palette.panel_border),
                BackgroundColor(palette.panel),
            ),
            M3Button::default(),
            action,
            PointerOnly,
            InstrumentPanel,
            RadiusToken(RadiusLevel::Medium),
            UiId::new(id),
        ))
        .with_children(|b| {
            let mut text = b.spawn((
                texts(key, TypeScale::BodyMedium, InstrumentInk::Text, palette),
                L10nKey(key),
            ));
            if action == ViewpointButton::Scope {
                text.insert(ViewpointScopeText);
            }
        });
}

/// One value row: marker, name, *minus*, the track with its fill, *plus* and the value.
fn value_row(
    p: &mut ChildSpawnerCommands,
    (row, name_key): (ViewpointRow, &'static str),
    palette: &InstrumentPalette,
) {
    p.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|r| {
        r.spawn((
            texts("›", TypeScale::BodyMedium, InstrumentInk::Needle, palette),
            ViewpointMarker(row),
            Node {
                width: Val::Px(10.0),
                ..default()
            },
        ));
        r.spawn((
            texts(name_key, TypeScale::Label, InstrumentInk::TextDim, palette),
            L10nKey(name_key),
            ViewpointName(row),
            Node {
                width: Val::Px(86.0),
                ..default()
            },
        ));
        small_button(
            r,
            ViewpointButton::Step(row, -1),
            keys::VIEWPOINT_DECREASE,
            format!("hud.viewpoint.{}.minus", row.id()),
            palette,
        );
        track(r, row, palette);
        small_button(
            r,
            ViewpointButton::Step(row, 1),
            keys::VIEWPOINT_INCREASE,
            format!("hud.viewpoint.{}.plus", row.id()),
            palette,
        );
        r.spawn((
            texts("", TypeScale::BodyMedium, InstrumentInk::Text, palette),
            ViewpointValue(row),
            UiId::new(format!("hud.viewpoint.{}.value", row.id())),
            Node {
                min_width: Val::Px(64.0),
                ..default()
            },
        ));
    });
}

/// The track of a value row: a clickable bar with the fill that shows the value.
fn track(r: &mut ChildSpawnerCommands, row: ViewpointRow, palette: &InstrumentPalette) {
    r.spawn((
        Node {
            width: Val::Px(92.0),
            height: Val::Px(12.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(palette.track),
        BorderColor::all(palette.panel_border),
        Button,
        PointerOnly,
        RelativeCursorPosition::default(),
        ViewpointTrack(row),
        ViewpointPaint(false),
        RadiusToken(RadiusLevel::Full),
        UiId::new(format!("hud.viewpoint.{}.track", row.id())),
    ))
    .with_children(|t| {
        t.spawn((
            Node {
                width: Val::Percent(50.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(palette.needle),
            ViewpointFill(row),
            ViewpointPaint(true),
            RadiusToken(RadiusLevel::Full),
        ));
    });
}

/// *Reset*, *Cancel* and *Save*.
fn action_buttons(p: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    p.spawn(Node {
        column_gap: Val::Px(8.0),
        justify_content: JustifyContent::FlexEnd,
        ..default()
    })
    .with_children(|b| {
        small_button(
            b,
            ViewpointButton::Reset,
            keys::VIEWPOINT_RESET,
            "hud.viewpoint.reset".to_owned(),
            palette,
        );
        small_button(
            b,
            ViewpointButton::Cancel,
            keys::VIEWPOINT_CANCEL,
            "hud.viewpoint.cancel".to_owned(),
            palette,
        );
        small_button(
            b,
            ViewpointButton::Save,
            keys::VIEWPOINT_SAVE,
            "hud.viewpoint.save".to_owned(),
            palette,
        );
    });
}

/// Spawns the panel.
pub fn spawn_panel(commands: &mut Commands, palette: &InstrumentPalette) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(68.0),
                    width: Val::Px(super::layout::VIEWPOINT_PANEL_WIDTH),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Large,
                palette,
            ),
            ViewpointPanel,
            GlobalZIndex(60),
            InGameEntity,
            UiId::new("hud.viewpoint"),
        ))
        .with_children(|p| {
            p.spawn((
                texts(
                    keys::VIEWPOINT_TITLE,
                    TypeScale::BodyLarge,
                    InstrumentInk::Needle,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_TITLE),
            ));
            p.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                ViewpointSubtitle,
                UiId::new("hud.viewpoint.subtitle"),
            ));
            for (row, name_key) in ROW_NAMES {
                value_row(p, (row, name_key), palette);
            }
            p.spawn((
                Node {
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                ViewpointScopeRow,
            ))
            .with_children(|r| {
                small_button(
                    r,
                    ViewpointButton::Scope,
                    keys::VIEWPOINT_SCOPE_VEHICLE,
                    "hud.viewpoint.scope".to_owned(),
                    palette,
                );
            });
            p.spawn((
                texts(
                    keys::VIEWPOINT_UNSAVED,
                    TypeScale::Label,
                    InstrumentInk::Warning,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_UNSAVED),
                ViewpointUnsaved,
                Visibility::Hidden,
            ));
            p.spawn((
                texts(
                    keys::VIEWPOINT_HINT,
                    TypeScale::Label,
                    InstrumentInk::TextDim,
                    palette,
                ),
                L10nKey(keys::VIEWPOINT_HINT),
            ));
            action_buttons(p, palette);
        });
}

mod params;
mod systems;
pub use systems::{panel_buttons, panel_tracks, sync_panel, update_panel, value_text};
