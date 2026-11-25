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
pub mod control_label;
pub mod details;
pub mod door_camera;
pub mod fade;
pub mod fuel_hud;
pub mod fullmap;
pub use opts_hud_keys as keys;
pub mod layout;
pub mod minimap;
pub mod mouse_driving;
mod params;
pub mod parts;
pub mod profile;
pub mod protocol;
pub mod route_path;
pub mod update;
pub mod values;
pub mod viewpoint;
pub mod virtual_mirrors;
pub use opts_hud_widgets as widget;

use bevy::prelude::*;

use crate::app_state::AppState;

mod schedule;
use schedule::{add_panel_systems, add_session_systems};

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

/// The HUD's profiles, values, layout, alert queue and messages.
fn add_resources(app: &mut App) {
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
        .init_resource::<opts_bevy::HoveredControl>()
        .init_resource::<opts_bevy::MouseDriving>()
        .init_resource::<AlertQueue>()
        .init_resource::<update::CameraLabel>()
        .init_resource::<crate::session_vehicles::PlaceLabel>()
        .init_resource::<details::DetailsStore>()
        .init_resource::<opts_bevy::ViewpointState>()
        .add_message::<RaiseAlert>()
        .add_message::<ClearAlert>()
        .add_message::<ShowDetailsToast>();
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
        add_resources(app);
        add_session_systems(app);
        add_panel_systems(app);
    }
}

#[cfg(test)]
mod tests;
