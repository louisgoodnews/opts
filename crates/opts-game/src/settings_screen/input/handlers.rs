//! The systems that handle the page's clicks, switches, combo choices and search field.

use bevy::prelude::*;

use crate::settings_registry::SettingsApi;
use crate::settings_screen::page::SettingsPageState;

use super::change::{change_from_page, step};
use super::ops::{run_op, select_category};
use super::params;
use super::{PendingRevert, SettingOp};

/// Handles clicks, switches and the search field.
pub fn handle_page_input(
    mut commands: Commands,
    events: params::PageEvents,
    controls: params::PageControls,
    mut state: ResMut<SettingsPageState>,
    mut api: SettingsApi,
    mut pending: Option<ResMut<PendingRevert>>,
    loc: Res<opts_gui::l10n::Localization>,
) {
    let params::PageEvents {
        mut clicks,
        mut toggles,
        mut texts,
        mut chosen,
        mut actions,
    } = events;
    let params::PageControls {
        combos,
        ops,
        switches,
        search,
    } = controls;
    for t in texts.read() {
        if search.contains(t.entity) && state.query != t.text {
            state.query.clone_from(&t.text);
            state.dirty = true;
        }
    }
    for ev in chosen.read() {
        let Ok(combo) = combos.get(ev.entity) else {
            continue;
        };
        if let Some(value) = combo.values.get(ev.selected_index) {
            if *value != api.text(combo.key) {
                change_from_page(
                    &mut api,
                    &mut commands,
                    &mut pending,
                    combo.key,
                    Some(value),
                );
            }
        }
    }
    for t in toggles.read() {
        if let Ok(s) = switches.get(t.entity) {
            let v = if t.is_on { "true" } else { "false" };
            change_from_page(&mut api, &mut commands, &mut pending, s.0, Some(v));
        }
    }
    for c in clicks.read() {
        let Ok(op) = ops.get(c.0) else { continue };
        match op {
            SettingOp::Set { key, value } => {
                change_from_page(&mut api, &mut commands, &mut pending, key, Some(value));
            }
            SettingOp::Step { key, dir } => {
                let next = api
                    .registry
                    .get(key)
                    .and_then(|d| step(d, &api.text(key), *dir));
                if let Some(v) = next {
                    change_from_page(&mut api, &mut commands, &mut pending, key, Some(&v));
                }
            }
            SettingOp::Reset { key } => {
                change_from_page(&mut api, &mut commands, &mut pending, key, None);
            }
            SettingOp::Run { key } => run_op(key, &mut state, &api, &mut actions, &loc),
            SettingOp::Category(cat) => select_category(&mut state, *cat),
        }
    }
}
