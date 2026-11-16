//! The HUD's system schedule: the session systems (sampling, layout, texts, fades, the rebuild
//! for another vehicle kind) and the panel systems (viewpoint, control label, mouse cue, mirrors,
//! detail toasts).

use super::*;

/// The systems that keep the HUD in step with the drive: sampling, layout, texts, fades and the
/// rebuild when another kind of vehicle becomes active.
pub(super) fn add_session_systems(app: &mut App) {
    app.add_systems(OnEnter(AppState::InGame), reset_session_state)
        .add_systems(
            Update,
            (door_camera::sync_setting, door_camera::update_overlay)
                .chain()
                .run_if(in_state(AppState::InGame))
                .run_if(resource_exists::<opts_bevy::door_camera::DoorCamera>),
        )
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
        );
}

/// The viewpoint panel, the control label, the mouse cue, the mirrors and the detail toasts.
pub(super) fn add_panel_systems(app: &mut App) {
    app.add_systems(
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
        (control_label::sync_label, mouse_driving::sync_cue)
            .run_if(in_state(AppState::InGame))
            .run_if(resource_exists::<opts_gui::prelude::MaterialTheme>)
            .run_if(resource_exists::<opts_gui::theme::InstrumentVariant>),
    )
    .add_systems(
        Update,
        virtual_mirrors::mirror_clicks
            .run_if(in_state(AppState::InGame))
            .run_if(resource_exists::<opts_bevy::mirrors::VirtualMirrors>),
    )
    .add_systems(
        PostUpdate,
        virtual_mirrors::sync_virtual_mirrors
            .run_if(in_state(AppState::InGame))
            .run_if(resource_exists::<opts_gui::prelude::MaterialTheme>)
            .run_if(resource_exists::<opts_gui::theme::InstrumentVariant>)
            .run_if(resource_exists::<opts_bevy::mirrors::VirtualMirrors>)
            .run_if(resource_exists::<opts_bevy::camera::CameraState>)
            .run_if(resource_exists::<opts_bevy::camera::CameraAvailability>),
    )
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
            details::open_details.before(opts_gui::components::toast::toast_action_click_system),
            details::forget_dismissed,
            details::details_choices,
        )
            // In order, with the commands applied in between: `forget_dismissed` must not
            // see a toast that `spawn_details_toasts` has only just queued as gone.
            .chain(),
    );
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
