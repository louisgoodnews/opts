//! Rebuilding the active vehicle menu page when state changes. The pages that are built from the
//! menu's state alone (main, select, change-map confirmation) are in [`pages`].

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::in_game::omsi_world::{EntryIndex, WorldLoadJob};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

use super::change_map_page::build_change_map;
use super::configs_page::build_configs;
use super::route_page::{build_jump, build_route, route_view};
use super::setvars_page::build_setvars;
use super::{ListScroll, MenuRoot, Page, VehicleMenu};

mod pages;

use pages::{confirm_page, main_page, select_page};

type Roots<'w, 's> = Query<'w, 's, Entity, With<MenuRoot>>;

/// What the menu pages are built from.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct RebuildData<'w, 's> {
    theme: Option<Res<'w, MaterialTheme>>,
    catalogue: Option<Res<'w, GameCatalogue>>,
    setup: Option<Res<'w, NewGameSetup>>,
    registry: Res<'w, SessionVehicles>,
    index: Res<'w, EntryIndex>,
    job: Option<Res<'w, WorldLoadJob>>,
    routes: Query<'w, 's, &'static opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>,
    env: Option<Res<'w, opts_bevy::environment::SessionEnvironment>>,
    session: Option<Res<'w, super::super::types::InGameSessionInfo>>,
}

/// Rebuilds the menu page when it or the data it shows changed.
pub(super) fn rebuild_menu(
    mut menu: ResMut<VehicleMenu>,
    mut commands: Commands,
    roots: Roots,
    scrolls: Query<(&ListScroll, &ScrollPosition)>,
    data: RebuildData,
) {
    let RebuildData {
        theme,
        catalogue,
        setup,
        registry,
        index,
        job,
        routes,
        env,
        session,
    } = data;
    let want = (menu.page, menu.config_open);
    // The Setvars page also follows changes made outside it (the control protocol).
    let shown_now = registry.active().map(|r| r.setvars.clone());
    let stale = menu.page == Page::Setvars && shown_now != menu.live_setvars.shown;
    if !menu.is_changed() && menu.built == Some(want) && !stale {
        return;
    }
    // Keep the scroll offsets of the lists across the rebuild.
    for (l, p) in &scrolls {
        menu.scroll[l.0] = p.y;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    menu.built = Some(want);
    // Remember what the page shows without calling it a change (that would rebuild once more).
    menu.bypass_change_detection().live_setvars.shown = shown_now;
    let Some(theme) = theme else { return };
    match menu.page {
        Page::Closed | Page::Formation => {}
        Page::Jump => {
            let entries = job.as_ref().map(|j| j.entry_records()).unwrap_or_default();
            build_jump(&mut commands, &menu, &entries, index.is_done());
        }
        Page::ChangeMap => {
            let Some(cat) = catalogue else { return };
            let current = session.as_ref().map_or("", |s| s.map_name.as_str());
            build_change_map(&mut commands, &menu, &cat, current);
        }
        Page::Setvars => {
            build_setvars(&mut commands, &menu, &registry);
        }
        Page::Configs => {
            let can_save = registry.active().is_some_and(|r| r.is_alive());
            build_configs(&mut commands, &menu, can_save);
        }
        Page::ChangeMapConfirm => {
            confirm_page(
                &mut commands,
                &menu,
                (catalogue, setup),
                (&registry, env.as_deref()),
            );
        }
        Page::Route => {
            let view = routes.iter().next().map(|r| route_view(&menu, &r.handle));
            build_route(&mut commands, &theme, &menu, view.as_ref());
        }
        Page::Main => main_page(&mut commands, &theme, &menu, &registry),
        Page::Select => select_page(
            &mut commands,
            &theme,
            &mut menu,
            (catalogue, setup.as_deref()),
            (job.as_deref(), &index),
        ),
    }
}
