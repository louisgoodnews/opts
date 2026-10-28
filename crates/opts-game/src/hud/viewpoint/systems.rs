//! Update, button, and track interaction systems for the driver viewpoint editor panel.

use super::*;

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
    let total = vehicle.single().map_or(0, |b| b.driver_camera_count());
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
        let want = [Display::None, Display::Flex][usize::from(state.model_key.is_some())];
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
