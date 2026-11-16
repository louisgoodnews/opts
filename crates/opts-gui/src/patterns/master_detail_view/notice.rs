//! Empty-state notice and search placeholder text of the master pane.

use bevy::prelude::*;

use super::chrome::set_if;
use crate::components::text_field::M3TextField;
use crate::l10n::{wizard_keys::md, L10nArgs, L10nKey, Localization};
use crate::patterns::master_detail::{MasterDetail, MasterDetailView};

pub(super) fn update_notice(
    nodes: &mut Query<&mut Node>,
    texts: &mut Query<(&mut L10nKey, &mut L10nArgs)>,
    model: &MasterDetail,
    view: &MasterDetailView,
) {
    let parts = view.parts;
    let empty_rows = !view.rows.iter().any(|r| r.is_item());
    if let Ok(mut n) = nodes.get_mut(parts.notice) {
        set_if(
            &mut n.display,
            if empty_rows {
                Display::Flex
            } else {
                Display::None
            },
        );
    }
    if let Ok((mut key, mut args)) = texts.get_mut(parts.notice_text) {
        if model.entries.is_empty() {
            set_if(&mut key.0, view.config.empty_key);
        } else {
            set_if(&mut key.0, md::NO_RESULTS);
            let a = L10nArgs::one("query", model.query.clone()); // l10n-ok: id or variable name
            if *args != a {
                *args = a;
            }
        }
    }
}

pub(super) fn update_placeholder(
    loc: &Localization,
    view: &MasterDetailView,
    model: &MasterDetail,
    text_fields: &mut Query<&mut M3TextField>,
) {
    let args = [("count".to_string(), model.entries.len().into())]; // l10n-ok: variable name
    let want = loc.t_args(view.config.search_key, &args);
    if let Ok(mut f) = text_fields.get_mut(view.parts.search) {
        if f.placeholder != want {
            f.placeholder = want;
        }
    }
}
