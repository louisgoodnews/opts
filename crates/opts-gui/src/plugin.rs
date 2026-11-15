//! Root plugin registering Material Design 3 GUI systems, events, and resources.

use bevy::prelude::*;

use crate::{
    interaction::{
        events::{
            ButtonClicked, CheckboxToggled, ComboboxSelected, DateSelected, DateTimeSelected,
            NotebookTabSelected, RadioSelected, SwitchToggled, TextChanged, ThemeChanged,
            ToastDismissed, ToplevelClosed, TreeNodeSelected, TreeNodeToggled,
        },
    },
    theme::{MaterialTheme, ThemeMode, ThemePreferences},
    tokens::{
        sync_background_roles, sync_border_roles, sync_foreground_roles, sync_radius_tokens,
        sync_typestyle_tokens,
    },
};

/// Main plugin providing the Material Design 3 GUI foundation for OPTS.
#[derive(Default)]
pub struct OptsGuiPlugin;

fn register_messages(app: &mut App) {
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
}

mod systems;

use systems::{register_form_systems, register_widget_systems};


impl Plugin for OptsGuiPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<MaterialTheme>() {
            app.init_resource::<MaterialTheme>();
        }

        app.add_plugins(crate::interaction::InteractionPlugin);
        crate::l10n::build(app);

        register_messages(app);

        crate::actions::build(app);
        app.add_plugins(crate::patterns::PatternsPlugin);

        // Bundled fonts: the UI family as the default text font, Material Icons as `ICON_FONT`
        app.add_systems(Startup, crate::fonts::install_fonts);

        // Theme state: preferences, modes, scale, density, layout class, motion
        app.add_plugins(crate::theme::ThemePlugin);

        // Modal dialogs and the multiline text area
        app.add_plugins((
            crate::components::modal::ModalPlugin,
            crate::components::text_area::TextAreaPlugin,
        ));

        // A `ThemeChanged` message becomes a preference change before the theme is resolved
        app.add_systems(
            PreUpdate,
            theme_change_listener.before(crate::theme::runtime::resolve_theme),
        );

        // Register core theme and token synchronization systems
        app.add_systems(
            Update,
            (
                sync_background_roles,
                sync_border_roles,
                sync_foreground_roles,
                sync_radius_tokens,
                sync_typestyle_tokens,
            ),
        );

        register_form_systems(app);
        register_widget_systems(app);
    }
}

/// System responding to [`ThemeChanged`] events: an explicit light/dark choice sets the theme
/// mode (kept for code that still sends the event; new code changes `ThemePreferences`).
fn theme_change_listener(
    mut events: MessageReader<ThemeChanged>,
    mut prefs: ResMut<ThemePreferences>,
) {
    for event in events.read() {
        let mode = if event.is_dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        if prefs.mode != mode || prefs.high_contrast {
            prefs.mode = mode;
            prefs.high_contrast = false;
        }
    }
}
