//! The combobox sync system: menu visibility, focus on open, and the displayed selection text.

use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocusVisible;
use bevy::prelude::*;

use super::{
    ComboboxLabelText, ComboboxMenu, ComboboxMenuItem, ComboboxReveal, ComboboxSearch, M3Combobox,
};
use crate::components::text_field::M3TextField;
use crate::interaction::{focus::FocusEvent, layer::FocusProxy};
use crate::l10n::Localization;

/// Parameters for [`combobox_sync_system`].
#[derive(SystemParam)]
pub struct ComboboxSyncParams<'w, 's> {
    pub visible: Option<Res<'w, InputFocusVisible>>,
    pub items: Query<'w, 's, (Entity, &'static ComboboxMenuItem)>,
    pub focus: MessageWriter<'w, FocusEvent>,
    pub loc: Option<Res<'w, Localization>>,
    pub combo_query: Query<'w, 's, (Entity, Ref<'static, M3Combobox>)>,
    pub menu_query: Query<'w, 's, (&'static ComboboxMenu, &'static mut Node)>,
    pub text_query: Query<'w, 's, (&'static ComboboxLabelText, &'static mut Text)>,
    pub searches: Query<
        'w,
        's,
        (
            &'static ComboboxSearch,
            &'static FocusProxy,
            &'static mut M3TextField,
        ),
    >,
}

/// System updating visibility of dropdown menu and updating display text. A list that opens
/// while keyboard focus is visible moves focus onto the selected (else first) item; a searchable
/// list always focuses its search field instead and scrolls the selected item into view.
pub fn combobox_sync_system(mut params: ComboboxSyncParams, mut commands: Commands) {
    let loc_changed = params.loc.as_ref().is_some_and(|l| l.is_changed());
    for (entity, combo) in &params.combo_query {
        if !combo.is_changed() && !loc_changed {
            continue;
        }
        if combo.is_changed() && combo.is_open {
            commands.entity(entity).try_insert(ComboboxReveal(0));
            let search = params.searches.iter().find(|(s, _, _)| s.0 == entity);
            if let Some((_, proxy, _)) = search {
                params.focus.write(FocusEvent::Set(proxy.0));
            } else if params.visible.as_ref().is_none_or(|v| v.0) {
                let wanted = combo.selected.unwrap_or(0);
                if let Some((item, _)) = params
                    .items
                    .iter()
                    .find(|(_, i)| i.combobox_entity == entity && i.index == wanted)
                {
                    params.focus.write(FocusEvent::Set(item));
                }
            }
        }
        if combo.is_changed() && !combo.is_open {
            for (s, _, mut field) in &mut params.searches {
                if s.0 == entity && !field.text.is_empty() {
                    field.text.clear();
                }
            }
        }
        for (menu, mut style) in &mut params.menu_query {
            if menu.0 == entity {
                style.display = if combo.is_open {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }

        let display_str = combo
            .selected
            .and_then(|idx| combo.options.get(idx))
            .cloned()
            .unwrap_or_else(|| {
                if combo.placeholder.is_empty() {
                    params
                        .loc
                        .as_ref()
                        .map(|l| l.t(crate::l10n::keys::COMBOBOX_PLACEHOLDER))
                        .unwrap_or_default()
                } else {
                    combo.placeholder.clone()
                }
            });

        for (lbl, mut text) in &mut params.text_query {
            if lbl.0 == entity {
                text.0 = display_str.clone();
            }
        }
    }
}
