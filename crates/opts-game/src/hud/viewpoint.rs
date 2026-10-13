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

/// Spawns the panel.
pub fn spawn_panel(commands: &mut Commands, palette: &InstrumentPalette) {
    commands
        .spawn((
            panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(140.0),
                    width: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
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
        });
}

/// Opens and closes the panel with the editor.
pub fn sync_panel(
    mut commands: Commands,
    state: Res<ViewpointState>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    panels: Query<Entity, With<ViewpointPanel>>,
) {
    let open = state.editing;
    if open && panels.is_empty() {
        spawn_panel(&mut commands, theme.instrument(*variant));
    } else if !open {
        for e in &panels {
            commands.entity(e).despawn();
        }
    }
}

/// The text of a row's number.
#[must_use]
pub fn value_text(state: &ViewpointState, row: ViewpointRow, loc: &Localization) -> String {
    let (key, value) = match row {
        ViewpointRow::X => (keys::VIEWPOINT_OFFSET, state.active.x * 100.0),
        ViewpointRow::Y => (keys::VIEWPOINT_OFFSET, state.active.y * 100.0),
        ViewpointRow::Z => (keys::VIEWPOINT_OFFSET, state.active.z * 100.0),
        ViewpointRow::Fov => (keys::VIEWPOINT_ANGLE, state.effective_fov_deg()),
    };
    let shown = if row == ViewpointRow::Fov {
        format!("{value:.0}")
    } else {
        format!("{:+.0}", value.round() + 0.0)
    };
    loc.t_args(key, &[("value".to_owned(), Arg::from(shown))])
}

/// Keeps the texts, bars and markers of the open panel in step with the state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_panel(
    state: Res<ViewpointState>,
    loc: Res<Localization>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    vehicle: Query<&BusVehicle, With<PlayerVehicle>>,
    mut values: Query<(&ViewpointValue, &mut Text), Without<ViewpointSubtitle>>,
    mut subtitle: Query<&mut Text, (With<ViewpointSubtitle>, Without<ViewpointValue>)>,
    mut fills: Query<(&ViewpointFill, &mut Node)>,
    mut markers: Query<(&ViewpointMarker, &mut Visibility), Without<ViewpointUnsaved>>,
    mut names: Query<(&ViewpointName, &mut InstrumentInk)>,
    mut unsaved: Query<&mut Visibility, (With<ViewpointUnsaved>, Without<ViewpointMarker>)>,
    mut paints: Query<(&ViewpointPaint, &mut BackgroundColor)>,
    mut scope_text: Query<&mut L10nKey, With<ViewpointScopeText>>,
    mut scope_row: Query<&mut Node, (With<ViewpointScopeRow>, Without<ViewpointFill>)>,
) {
    let palette = theme.instrument(*variant);
    for (paint, mut bg) in &mut paints {
        let want = if paint.0 {
            palette.needle
        } else {
            palette.track
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
    if !state.is_changed() && !loc.is_changed() {
        return;
    }
    for (v, mut text) in &mut values {
        let new = value_text(&state, v.0, &loc);
        if text.0 != new {
            text.0 = new;
        }
    }
    let total = vehicle.single().map_or(0, |b| b.driver_cameras.len());
    for mut text in &mut subtitle {
        let new = loc.t_args(
            keys::VIEWPOINT_SUBTITLE,
            &[
                ("index".to_owned(), Arg::from(state.camera_index + 1)),
                ("total".to_owned(), Arg::from(total.max(1))),
            ],
        );
        if text.0 != new {
            text.0 = new;
        }
    }
    for mut key in &mut scope_text {
        let want = L10nKey(if state.scope_model {
            keys::VIEWPOINT_SCOPE_MODEL
        } else {
            keys::VIEWPOINT_SCOPE_VEHICLE
        });
        if *key != want {
            *key = want;
        }
    }
    for mut node in &mut scope_row {
        let want = if state.model_key.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
    for (f, mut node) in &mut fills {
        let want = Val::Percent(state.fraction(f.0) * 100.0);
        if node.width != want {
            node.width = want;
        }
    }
    for (m, mut vis) in &mut markers {
        let want = if m.0 == state.row {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (n, mut ink) in &mut names {
        let want = if n.0 == state.row {
            InstrumentInk::Text
        } else {
            InstrumentInk::TextDim
        };
        if *ink != want {
            *ink = want;
        }
    }
    for mut vis in &mut unsaved {
        let want = if state.is_dirty() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// The buttons of the panel.
pub fn panel_buttons(
    mut clicks: MessageReader<ButtonClicked>,
    buttons: Query<&ViewpointButton>,
    mut state: ResMut<ViewpointState>,
) {
    for ButtonClicked(entity) in clicks.read() {
        let Ok(action) = buttons.get(*entity) else {
            continue;
        };
        if !state.editing {
            continue;
        }
        match *action {
            ViewpointButton::Step(row, dir) => {
                state.row = row;
                state.adjust(row, f32::from(dir));
            }
            ViewpointButton::Reset => state.reset(),
            ViewpointButton::Cancel => state.cancel(),
            ViewpointButton::Save => state.save(),
            ViewpointButton::Scope => {
                if state.model_key.is_some() {
                    state.scope_model = !state.scope_model;
                }
            }
        }
    }
}

/// Click or drag on a track sets the row to the position under the cursor.
pub fn panel_tracks(
    tracks: Query<(&ViewpointTrack, &RelativeCursorPosition), With<Pressed>>,
    mut state: ResMut<ViewpointState>,
) {
    if !state.editing {
        return;
    }
    for (track, cursor) in &tracks {
        if let Some(pos) = cursor.normalized {
            state.row = track.0;
            state.set_fraction(track.0, pos.x + 0.5);
        }
    }
}
