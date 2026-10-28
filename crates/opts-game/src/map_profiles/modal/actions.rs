//! Button, text-field, and cancel handlers for the *Map configuration* modal.

use opts_gui::components::modal::ModalEvent;
use opts_gui::interaction::events::{ButtonClicked, TextChanged};
use opts_gui::l10n::Localization;

use super::build::arg;
use super::*;
use crate::map_profiles::ProfileSource;

/// Applies the clicked button.
pub fn actions(
    mut clicks: MessageReader<ButtonClicked>,
    buttons: Query<&Action>,
    mut menu: ResMut<MapConfigMenu>,
    mut commands: Commands,
) {
    let todo: Vec<Action> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| buttons.get(*e).ok().cloned())
        .collect();
    for a in todo {
        match a {
            Action::Select(c) => menu.selected = Some(c),
            Action::Close => menu.open = false,
            Action::Apply { keep } => {
                let Some(choice) = menu.selected.clone() else {
                    continue;
                };
                commands.queue(move |w: &mut World| {
                    let status = match service::apply(w, &choice, keep) {
                        Ok(()) => w.resource::<Localization>().t("map-config-status-applied"),
                        Err(e) => w
                            .resource::<Localization>()
                            .t_args("map-config-status-error", &[arg("error", e)]),
                    };
                    w.resource_mut::<MapConfigMenu>().status = status;
                });
            }
            Action::Create { chrono } => {
                let name = menu.name.trim().to_string();
                let name = if name.is_empty() {
                    "Profile".to_string()
                } else {
                    name
                };
                commands.queue(move |w: &mut World| {
                    let loc = w.resource::<Localization>().clone();
                    let status = match service::create_from_current(w, &name, chrono) {
                        Ok(n) => loc.t_args("map-config-status-created", &[arg("name", n)]),
                        Err(e) => loc.t_args("map-config-status-error", &[arg("error", e)]),
                    };
                    let mut menu = w.resource_mut::<MapConfigMenu>();
                    menu.status = status;
                    menu.selected = Some(Choice::Saved(name));
                });
            }
            Action::Delete => {
                commands.queue(move |w: &mut World| {
                    let loc = w.resource::<Localization>().clone();
                    let chrono = service::view(w)
                        .is_some_and(|v| v.resolved.source == ProfileSource::Chrono);
                    let status = match service::delete(w, chrono) {
                        Ok(true) => loc.t("map-config-status-deleted"),
                        Ok(false) => loc.t("map-config-status-nothing"),
                        Err(e) => loc.t_args("map-config-status-error", &[arg("error", e)]),
                    };
                    let mut menu = w.resource_mut::<MapConfigMenu>();
                    menu.status = status;
                    menu.selected = None;
                });
            }
        }
    }
}

/// Keeps the typed name.
pub fn track_name(
    mut edited: MessageReader<TextChanged>,
    fields: Query<&NameField>,
    mut menu: ResMut<MapConfigMenu>,
) {
    for t in edited.read() {
        if fields.get(t.entity).is_ok() && menu.name != t.text {
            menu.name.clone_from(&t.text);
        }
    }
}

/// Escape closes the modal.
pub fn close_on_cancel(
    mut events: MessageReader<ModalEvent>,
    roots: Query<Entity, With<MapConfigRoot>>,
    mut menu: ResMut<MapConfigMenu>,
) {
    for ModalEvent::CancelRequested(e) in events.read() {
        if roots.contains(*e) {
            menu.open = false;
        }
    }
}
