//! The system registration of the plugin: form controls, widgets and the search of long menus.

use bevy::prelude::*;

use crate::{
    components::{
        button::button_click_system,
        checkbox::{checkbox_sync_system, checkbox_toggle_system},
        combobox::{
            combobox_arrow_system, combobox_header_system, combobox_item_system,
            combobox_sync_system,
        },
        date_select::date_select_system,
        datetime_select::datetime_select_system,
        notebook::{
            notebook_action_system, notebook_focus_selects_system, notebook_sync_system,
            notebook_tab_click_system,
        },
        radio::{radio_select_system, radio_sync_system},
        switch::{switch_sync_system, switch_toggle_system},
        text_field::{
            text_field_chrome_system, text_field_clear_system, text_field_cursor_style_system,
            text_field_disabled_system, text_field_sync_system,
        },
        toast::{toast_action_click_system, toast_timer_system},
        toplevel::toplevel_close_system,
        treeview::{treeview_arrow_system, treeview_click_system, treeview_sync_system},
    },
    interaction::{activation::ActivationSet, focus::focus_event_handler_system},
};

pub(super) fn register_form_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            button_click_system,
            date_select_system,
            datetime_select_system,
            checkbox_toggle_system,
            checkbox_sync_system,
            switch_toggle_system,
            switch_sync_system,
            radio_select_system,
            radio_sync_system,
        )
            .after(ActivationSet),
    );
    app.add_systems(
        Update,
        (
            text_field_sync_system,
            text_field_chrome_system,
            text_field_clear_system,
            text_field_disabled_system,
            text_field_cursor_style_system,
        )
            .after(ActivationSet),
    );
}

pub(super) fn register_widget_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            combobox_header_system,
            combobox_arrow_system,
            combobox_item_system,
            combobox_sync_system,
            notebook_tab_click_system,
            notebook_action_system,
            notebook_focus_selects_system,
            notebook_sync_system,
        )
            .after(focus_event_handler_system),
    );
    app.add_systems(
        Update,
        (
            toast_timer_system,
            crate::components::toast_stack::stack_toasts,
            toast_action_click_system,
            toplevel_close_system,
            treeview_click_system,
            treeview_arrow_system,
            treeview_sync_system,
        )
            .after(ActivationSet),
    );
    app.add_systems(
        Update,
        crate::components::combobox::combobox_search_systems().after(ActivationSet),
    );
}
