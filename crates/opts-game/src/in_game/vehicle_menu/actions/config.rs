//! The bus configuration page and the set-variable editor of the menu actions.

use super::*;

/// The bus configurations and the setvar editor. Returns the action when this group does not handle it.
pub(super) fn config_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::OpenConfigs => {
            c.menu.page = Page::Configs;
            c.menu.status.clear();
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                c.menu.configs = super::load_view(lib, cat);
            }
            None
        }
        Action::SaveConfig => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::save_active(c.menu, lib, c.registry, cat);
            }
            None
        }
        Action::SpawnConfig(id) => {
            if let (Some(cat), Some(job)) = (c.catalogue, c.job) {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::spawn(c.menu, lib, c.registry, cat, job, &id);
            }
            None
        }
        Action::OpenSetvars => {
            c.commands
                .queue(|w: &mut World| super::setvars_page::open(w));
            None
        }
        Action::LiveToggle(name) => {
            c.commands
                .queue(move |w: &mut World| super::setvars_page::change(w, &name, None));
            None
        }
        Action::LiveStep(name, delta) => {
            c.commands
                .queue(move |w: &mut World| super::setvars_page::change(w, &name, Some(delta)));
            None
        }
        Action::LiveReset => {
            c.commands
                .queue(|w: &mut World| super::setvars_page::reset(w));
            None
        }
        Action::RenameConfig(id) => {
            super::configs_page::start_rename(c.menu, &id);
            None
        }
        Action::CancelRename => {
            c.menu.configs.renaming = None;
            None
        }
        Action::CommitRename => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::commit_rename(c.menu, lib, cat);
            }
            None
        }
        Action::DeleteConfig(id) => {
            if let Some(cat) = c.catalogue {
                let lib = lib_of(c.library, c.fallback);
                super::configs_page::delete(c.menu, lib, cat, &id);
            }
            None
        }
        other => Some(other),
    }
}
