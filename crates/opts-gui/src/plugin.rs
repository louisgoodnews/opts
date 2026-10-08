//! Root plugin registering Material Design 3 GUI systems, events, and resources.

use bevy::prelude::*;

use crate::{
    components::{
        button::button_click_system,
        checkbox::{checkbox_sync_system, checkbox_toggle_system},
        combobox::{combobox_header_system, combobox_item_system, combobox_sync_system},
        date_select::date_select_system,
        datetime_select::datetime_select_system,
        notebook::{notebook_sync_system, notebook_tab_click_system},
        radio::{radio_select_system, radio_sync_system},
        switch::{switch_sync_system, switch_toggle_system},
        text_field::{text_field_change_system, text_field_interaction_system},
        toast::{toast_action_click_system, toast_timer_system},
        toplevel::toplevel_close_system,
        treeview::{treeview_click_system, treeview_sync_system},
    },
    interaction::{
        events::{
            ButtonClicked, CheckboxToggled, ComboboxSelected, DateSelected, DateTimeSelected,
            NotebookTabSelected, RadioSelected, SwitchToggled, TextChanged, ThemeChanged,
            ToastDismissed, ToplevelClosed, TreeNodeSelected, TreeNodeToggled,
        },
        focus::{focus_event_handler_system, keyboard_focus_navigation_system, FocusEvent},
    },
    theme::{ColorScheme, MaterialTheme},
    tokens::{
        sync_background_roles, sync_border_roles, sync_foreground_roles, sync_radius_tokens,
        sync_typestyle_tokens,
    },
};

/// Main plugin providing the Material Design 3 GUI foundation for OPTS.
#[derive(Default)]
pub struct OptsGuiPlugin;

impl Plugin for OptsGuiPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<MaterialTheme>() {
            app.init_resource::<MaterialTheme>();
        }

        // Register widget event streams
        app.add_event::<ButtonClicked>()
            .add_event::<CheckboxToggled>()
            .add_event::<SwitchToggled>()
            .add_event::<RadioSelected>()
            .add_event::<TextChanged>()
            .add_event::<DateSelected>()
            .add_event::<DateTimeSelected>()
            .add_event::<ComboboxSelected>()
            .add_event::<NotebookTabSelected>()
            .add_event::<ToastDismissed>()
            .add_event::<ToplevelClosed>()
            .add_event::<TreeNodeSelected>()
            .add_event::<TreeNodeToggled>()
            .add_event::<ThemeChanged>()
            .add_event::<FocusEvent>();

        // Register focus navigation systems
        app.add_systems(
            Update,
            (keyboard_focus_navigation_system, focus_event_handler_system),
        );

        // Register core theme and token synchronization systems
        app.add_systems(
            Update,
            (
                theme_change_listener,
                sync_background_roles,
                sync_border_roles,
                sync_foreground_roles,
                sync_radius_tokens,
                sync_typestyle_tokens,
            ),
        );

        // Register form and input interaction systems
        app.add_systems(
            Update,
            (
                button_click_system,
                date_select_system,
                datetime_select_system,
                text_field_interaction_system,
                text_field_change_system,
                checkbox_toggle_system,
                checkbox_sync_system,
                switch_toggle_system,
                switch_sync_system,
                radio_select_system,
                radio_sync_system,
            ),
        );

        // Register advanced navigation and container widget systems
        app.add_systems(
            Update,
            (
                combobox_header_system,
                combobox_item_system,
                combobox_sync_system,
                notebook_tab_click_system,
                notebook_sync_system,
                toast_timer_system,
                toast_action_click_system,
                toplevel_close_system,
                treeview_click_system,
                treeview_sync_system,
            ),
        );
    }
}

/// System responding to [`ThemeChanged`] events by updating [`MaterialTheme`].
fn theme_change_listener(mut events: EventReader<ThemeChanged>, mut theme: ResMut<MaterialTheme>) {
    for event in events.read() {
        theme.scheme = if event.is_dark {
            ColorScheme::dark()
        } else {
            ColorScheme::light()
        };
    }
}
