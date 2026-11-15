//! The menu's plugin: its systems, the control protocol's command queue, opening and closing.

use super::*;

/// Commands for the menu from outside the UI (the control protocol's `menu.do`): `open`,
/// `close`, `spawn` (the Select Vehicle page), `vehicle <text>`, `repaint <text>`,
/// `entry <text>`, `config`, `toggle <variable>`, `confirm`, `despawn`, `back`. The texts match
/// names case-insensitively by substring.
#[derive(Resource, Default)]
pub struct MenuCommands(pub Vec<String>);

/// Registers the menu.
#[derive(Default)]
pub struct VehicleMenuPlugin;

impl Plugin for VehicleMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleMenu>()
            .init_resource::<MenuModalActive>()
            .init_resource::<MenuCommands>()
            .init_resource::<EntryIndex>()
            .init_resource::<crate::new_game_screen::thumbnails::ThumbnailService>()
            .init_resource::<crate::new_game_screen::livery::LiveryState>()
            .init_resource::<crate::new_game_screen::GameCatalogue>()
            .add_systems(
                Update,
                (
                    toggle_menu,
                    search_input,
                    start_entry_index,
                    menu_actions,
                    refresh_entries,
                    refresh_session_rows,
                    rebuild_menu,
                    // The picture slots of the Configurations page (M-0077).
                    crate::new_game_screen::thumbnails::fill_thumbnails
                        .run_if(|menu: Res<VehicleMenu>| menu.is_open()),
                    scroll_lists,
                )
                    .chain()
                    .run_if(in_state(crate::AppState::InGame)),
            )
            .add_systems(OnEnter(crate::AppState::InGame), reset_menu)
            .add_systems(OnExit(crate::AppState::InGame), reset_menu);
    }
}

fn reset_menu(
    mut menu: ResMut<VehicleMenu>,
    mut entries: ResMut<EntryIndex>,
    mut active: ResMut<MenuModalActive>,
) {
    *menu = VehicleMenu::default();
    entries.reset();
    active.0 = false;
}

/// The optional gates that keep the menu's input gate up or block it from opening.
#[derive(bevy::ecs::system::SystemParam)]
struct ToggleGates<'w> {
    report: Option<Res<'w, ModalActive>>,
    pause: Option<Res<'w, crate::pause::Pause>>,
    fullmap: Option<Res<'w, crate::hud::fullmap::FullMapState>>,
}

/// Left Alt opens and closes the menu; Escape steps back (the modals emit it).
fn toggle_menu(
    keys: Res<ButtonInput<KeyCode>>,
    load: Res<SimulationLoadState>,
    mut menu: ResMut<VehicleMenu>,
    mut active: ResMut<MenuModalActive>,
    mut cancels: MessageReader<ModalEvent>,
    mut grace: Local<u8>,
    gates: ToggleGates,
) {
    let ToggleGates {
        report,
        pause,
        fullmap,
    } = gates;
    let mut step_back = false;
    for ModalEvent::CancelRequested(_) in cancels.read() {
        step_back = true;
    }
    if step_back && menu.page != Page::Closed {
        if menu.config_open {
            menu.config_open = false;
        } else if menu.configs.renaming.take().is_some() {
            // Escape leaves the name field first.
        } else if menu.page == Page::ChangeMapConfirm {
            menu.page = Page::ChangeMap;
        } else if menu.page == Page::Formation {
            menu.page = Page::Select;
        } else if matches!(
            menu.page,
            Page::Select
                | Page::Route
                | Page::Jump
                | Page::ChangeMap
                | Page::Configs
                | Page::Setvars
        ) {
            menu.page = Page::Main;
        } else {
            menu.page = Page::Closed;
        }
    } else if keys.just_pressed(KeyCode::AltLeft)
        && load.is_complete
        && !report.is_some_and(|m| m.0)
    {
        if menu.page == Page::Closed {
            menu.page = Page::Main;
            menu.status.clear();
        } else {
            menu.page = Page::Closed;
            menu.config_open = false;
        }
    }
    // The gate stays up for one more frame after closing, so the Escape press that closed the
    // menu does not also leave the session.
    if menu.page != Page::Closed {
        *grace = 2;
    } else if *grace > 0 {
        *grace -= 1;
    }
    let open =
        *grace > 0 || pause.is_some_and(|p| p.open) || fullmap.is_some_and(|m| m.holds_input());
    if active.0 != open {
        active.0 = open;
    }
}

/// Starts resolving the entry points when the menu is first needed.
fn start_entry_index(
    load: Res<SimulationLoadState>,
    job: Option<Res<WorldLoadJob>>,
    mut entries: ResMut<EntryIndex>,
) {
    // Started as soon as the session is up, so the index is ready by the time the menu opens.
    if !load.is_complete {
        return;
    }
    if let Some(job) = job {
        entries.start(&job);
    }
    entries.poll();
}
