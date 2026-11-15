//! The settings page systems: focus return, category styling, the search field, and the mouse wheel on the pane.

use super::*;

/// Puts keyboard focus back on the control that had it before the pane was rebuilt.
pub fn restore_focus_system(
    mut state: ResMut<SettingsPageState>,
    ids: Query<(Entity, &UiId)>,
    mut focus: MessageWriter<opts_gui::interaction::focus::FocusEvent>,
) {
    let Some(want) = state.restore_focus.clone() else {
        return;
    };
    if let Some((e, _)) = ids.iter().find(|(_, u)| u.0 == want.as_str()) {
        focus.write(opts_gui::interaction::focus::FocusEvent::Set(e));
        state.restore_focus = None;
    }
}

/// Marks the selected category button and dims the others.
pub fn style_categories(
    state: Res<SettingsPageState>,
    mut buttons: Query<(&SettingOp, &mut BackgroundRole, Option<&Children>)>,
    mut labels: Query<&mut ForegroundRole>,
    fresh: Query<(), Added<SettingOp>>,
) {
    if !state.is_changed() && fresh.is_empty() {
        return;
    }
    let searching = !state.query.trim().is_empty();
    for (op, mut bg, kids) in &mut buttons {
        let SettingOp::Category(cat) = op else {
            continue;
        };
        let selected = !searching && *cat == state.category;
        bg.0 = if selected {
            ColorRole::Primary
        } else {
            ColorRole::Surface
        };
        for k in kids.into_iter().flatten() {
            if let Ok(mut fg) = labels.get_mut(*k) {
                fg.0 = if selected {
                    ColorRole::OnPrimary
                } else {
                    ColorRole::Primary
                };
            }
        }
    }
}

/// Gives the search field its localized placeholder and name, a usable width, and empties it
/// after a category was chosen.
pub fn search_field_system(
    mut state: ResMut<SettingsPageState>,
    loc: Res<Localization>,
    mut fields: Query<(&mut M3TextField, &mut Node), With<SettingsSearch>>,
    fresh: Query<(), Added<SettingsSearch>>,
) {
    let fresh = !fresh.is_empty();
    for (mut field, mut node) in &mut fields {
        let want = loc.t("settings-search-placeholder");
        if field.placeholder != want {
            field.placeholder.clone_from(&want);
            field.label = want;
        }
        if fresh {
            node.min_width = Val::Px(260.0);
            node.flex_grow = 0.0;
        }
        if state.clear_search_field {
            field.text.clear();
        }
    }
    if state.clear_search_field {
        state.clear_search_field = false;
    }
}

/// Mouse wheel over the pane scrolls it.
pub fn scroll_pane(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut panes: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<PagePane>>,
) {
    let Ok(window) = window.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else {
        wheel.clear();
        return;
    };
    for e in wheel.read() {
        for (mut pos, transform, node) in &mut panes {
            if !node.contains_point(*transform, cursor) {
                continue;
            }
            let max =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
            pos.y = (pos.y - e.y * 36.0).clamp(0.0, max);
        }
    }
}
