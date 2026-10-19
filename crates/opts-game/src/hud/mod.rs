//! The HUD: widgets from a registry, placed by profiles (`docs/src/architecture/hud-registry.md`).
//!
//! * [`widget`]: slots, vehicle kinds and the [`HudWidget`] descriptors.
//! * [`profile`]: RON profiles per vehicle kind, with user overrides.
//! * [`build`]: spawns the HUD of the player's vehicle; [`update`]: change-driven updates.
//! * [`alerts`]: the one-at-a-time alert line (UI-P13); [`fade`]: status bar, hints and the
//!   1.5 s camera label; [`details`]: toasts with a *Details* dialog (Copy, Report).
//!
//! The HUD never takes input focus, never moves, and never covers the middle of the view.

pub mod alerts;
pub mod build;
pub mod details;
pub mod fade;
pub mod fullmap;
pub mod keys;
pub mod layout;
pub mod minimap;
pub mod parts;
pub mod profile;
pub mod protocol;
pub mod update;
pub mod values;
pub mod viewpoint;
pub mod widget;

use bevy::prelude::*;

use crate::app_state::AppState;

pub use alerts::{AlertQueue, AlertSeverity, ClearAlert, RaiseAlert};
pub use details::ShowDetailsToast;
pub use profile::{HudProfile, HudProfiles};
pub use values::HudValues;
pub use widget::{HudWidget, Row, Slot, VehicleKind, WIDGETS};

/// Merges the HUD message tables into the localisation.
pub fn add_messages(loc: &mut opts_gui::l10n::Localization) {
    for (locale, source) in [
        ("en", include_str!("l10n/hud-en.ftl")),
        ("de", include_str!("l10n/hud-de.ftl")),
    ] {
        for problem in loc.extend(locale, source) {
            tracing::warn!("HUD message table ({locale}): {problem}");
        }
    }
}

/// Registers the HUD resources, messages and systems.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<opts_gui::l10n::Localization>()
        {
            app.insert_resource(opts_gui::l10n::Localization::detect());
        }
        add_messages(
            &mut app
                .world_mut()
                .resource_mut::<opts_gui::l10n::Localization>(),
        );
        app.insert_resource(HudProfiles::load(profile::user_dir().as_deref()))
            .init_resource::<HudValues>()
            .init_resource::<layout::HudLayout>()
            .init_resource::<opts_gui::components::toast_stack::ToastInset>()
            .add_systems(
                Update,
                layout::toast_inset.before(opts_gui::components::toast_stack::stack_toasts),
            )
            .add_plugins(minimap::MiniMapPlugin)
            .add_plugins(fullmap::FullMapPlugin)
            .init_resource::<AlertQueue>()
            .init_resource::<update::CameraLabel>()
            .init_resource::<crate::session_vehicles::PlaceLabel>()
            .init_resource::<details::DetailsStore>()
            .init_resource::<opts_bevy::ViewpointState>()
            .add_message::<RaiseAlert>()
            .add_message::<ClearAlert>()
            .add_message::<ShowDetailsToast>()
            .add_systems(OnEnter(AppState::InGame), reset_session_state)
            .add_systems(
                Update,
                (
                    values::sample_values,
                    (layout::measure, layout::apply, layout::place_camera_label).chain(),
                    (
                        update::tick_session_clock,
                        update::camera_label_changes,
                        alerts::parking_brake_alert,
                        alerts::update_queue,
                        fade::status_follows_pause,
                    ),
                    (
                        update::update_texts,
                        update::update_widget_display,
                        update::update_alert_slot,
                        fade::drive_fades,
                    ),
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                rebuild_for_kind
                    .after(values::sample_values)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    viewpoint::update_panel,
                    viewpoint::panel_buttons,
                    viewpoint::panel_tracks,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(opts_bevy::reporting::not_modal_active)
                    .run_if(resource_exists::<opts_gui::prelude::MaterialTheme>)
                    .run_if(resource_exists::<opts_gui::theme::InstrumentVariant>),
            )
            // The panel is spawned after the frame's input, in `PostUpdate`: a spawn system in
            // `Update` would add a sync point that reorders the toast systems below.
            .add_systems(
                PostUpdate,
                viewpoint::sync_panel
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<opts_gui::prelude::MaterialTheme>)
                    .run_if(resource_exists::<opts_gui::theme::InstrumentVariant>),
            )
            .add_systems(
                Update,
                (
                    details::spawn_details_toasts,
                    // Before the toast's own click handler despawns the toast with the action
                    // button that was clicked.
                    details::open_details
                        .before(opts_gui::components::toast::toast_action_click_system),
                    details::forget_dismissed,
                    details::details_choices,
                )
                    // In order, with the commands applied in between: `forget_dismissed` must not
                    // see a toast that `spawn_details_toasts` has only just queued as gone.
                    .chain(),
            );
    }
}

/// The HUD follows the active vehicle: when another kind of vehicle becomes active the HUD is
/// rebuilt with the profile of that kind.
fn rebuild_for_kind(
    mut commands: Commands,
    values: Res<HudValues>,
    active: Option<Res<build::ActiveProfile>>,
    roots: Query<Entity, With<build::HudRoot>>,
    theme: Option<Res<opts_gui::prelude::MaterialTheme>>,
) {
    let (Some(active), Some(theme)) = (active, theme) else {
        return;
    };
    if !values.has_vehicle || active.kind == values.kind {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    build::queue_spawn(&mut commands, &theme);
}

/// A new drive starts without the alerts, label and values of the last one.
fn reset_session_state(
    mut values: ResMut<HudValues>,
    mut queue: ResMut<AlertQueue>,
    mut label: ResMut<update::CameraLabel>,
) {
    *values = HudValues::default();
    *queue = AlertQueue::default();
    *label = update::CameraLabel::default();
}

#[cfg(test)]
mod tests;
