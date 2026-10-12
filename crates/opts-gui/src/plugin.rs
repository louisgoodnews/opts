//! Root plugin registering Material Design 3 GUI systems, events, and resources.

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
    interaction::{
        activation::ActivationSet,
        events::{
            ButtonClicked, CheckboxToggled, ComboboxSelected, DateSelected, DateTimeSelected,
            NotebookTabSelected, RadioSelected, SwitchToggled, TextChanged, ThemeChanged,
            ToastDismissed, ToplevelClosed, TreeNodeSelected, TreeNodeToggled,
        },
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

        app.add_plugins(crate::interaction::InteractionPlugin);

        // Register widget event streams
        app.add_message::<ButtonClicked>()
            .add_message::<CheckboxToggled>()
            .add_message::<SwitchToggled>()
            .add_message::<RadioSelected>()
            .add_message::<TextChanged>()
            .add_message::<DateSelected>()
            .add_message::<DateTimeSelected>()
            .add_message::<ComboboxSelected>()
            .add_message::<NotebookTabSelected>()
            .add_message::<ToastDismissed>()
            .add_message::<ToplevelClosed>()
            .add_message::<TreeNodeSelected>()
            .add_message::<TreeNodeToggled>()
            .add_message::<ThemeChanged>();

        crate::actions::build(app);

        // Bundled fonts: DejaVu Sans as the default text font, Material Icons as `IconFont`
        app.add_systems(Startup, crate::fonts::install_fonts);

        // Modal dialogs and the multiline text area
        app.add_plugins((
            crate::components::modal::ModalPlugin,
            crate::components::text_area::TextAreaPlugin,
        ));

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
                text_field_sync_system,
                text_field_chrome_system,
                text_field_clear_system,
                text_field_disabled_system,
                text_field_cursor_style_system,
                checkbox_toggle_system,
                checkbox_sync_system,
                switch_toggle_system,
                switch_sync_system,
                radio_select_system,
                radio_sync_system,
            )
                .after(ActivationSet),
        );

        // Register advanced navigation and container widget systems
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
                toast_timer_system,
                toast_action_click_system,
                toplevel_close_system,
                treeview_click_system,
                treeview_arrow_system,
                treeview_sync_system,
            )
                .after(ActivationSet),
        );
    }
}

/// System responding to [`ThemeChanged`] events by updating [`MaterialTheme`].
fn theme_change_listener(
    mut events: MessageReader<ThemeChanged>,
    mut theme: ResMut<MaterialTheme>,
) {
    for event in events.read() {
        theme.scheme = if event.is_dark {
            ColorScheme::dark()
        } else {
            ColorScheme::light()
        };
    }
}
