//! The *Change map* pages of the Alt menu (M-0108): the list of installed maps, then a page that
//! shows what the change does in [`confirm`] (start point, vehicle, time, dropped vehicles) and
//! asks once.

use opts_bevy::environment::SessionEnvironment;

use super::build::{button, label, scroll_list};
use super::*;
use crate::in_game::change_map::{
    self, inputs_from, session_clock, ChangeMapError, ChangeMapPlan, VehicleChoice,
};

mod confirm;

pub(super) use confirm::build_change_map_confirm;

/// What the player has chosen on the pages so far.
#[derive(Debug, Clone, Default)]
pub(super) struct ChangeMapDraft {
    /// Folder name of the target map.
    pub map: Option<String>,
    /// The start point asked for (`None`: the map's first).
    pub entry: Option<String>,
    /// Go on without a vehicle.
    pub without_vehicle: bool,
}

impl ChangeMapDraft {
    pub(super) fn choice(&self) -> VehicleChoice {
        if self.without_vehicle {
            VehicleChoice::None
        } else {
            VehicleChoice::Carry
        }
    }
}

/// The plan of the draft, `None` while no map is chosen.
pub(super) fn draft_plan(
    menu: &VehicleMenu,
    cat: &GameCatalogue,
    registry: &SessionVehicles,
    env: Option<&SessionEnvironment>,
    setup: Option<&NewGameSetup>,
) -> Option<Result<ChangeMapPlan, ChangeMapError>> {
    let map = menu.change_map.map.as_deref()?;
    let inputs = inputs_from(cat, Some(registry), session_clock(env, setup));
    Some(change_map::plan(
        &inputs,
        map,
        menu.change_map.entry.as_deref(),
        menu.change_map.choice(),
    ))
}

/// The list of installed maps.
pub(super) fn build_change_map(
    commands: &mut Commands,
    menu: &VehicleMenu,
    cat: &GameCatalogue,
    current: &str,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 80.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Change map", 22.0, ColorRole::Primary);
        label(
            p,
            "The session starts again on the map you choose; your vehicle and the time stay.",
            13.0,
            ColorRole::OnSurfaceVariant,
        );
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        scroll_list(p, 2, menu.scroll[2], Val::Percent(100.0), |list| {
            for m in &cat.maps {
                let here = m.friendly_name == current || m.name == current || m.id == current;
                let text = if here {
                    format!("{} (current)", m.friendly_name)
                } else {
                    m.friendly_name.clone()
                };
                let b = button(list, &text, here, Action::PickMap(m.id.clone()));
                list.commands()
                    .entity(b)
                    .insert(UiId::new(format!("vehicle.map.{}", m.id)));
            }
        });
        button(p, "Back", false, Action::Back);
    });
}

/// Starts the change the draft describes (a click on *Change map*, `menu.do confirm`).
pub(super) fn confirm(world: &mut World) {
    let Some(draft) = world
        .get_resource::<VehicleMenu>()
        .map(|m| m.change_map.clone())
    else {
        return;
    };
    let Some(map) = draft.map.clone() else {
        return;
    };
    match change_map::plan_in_world(world, &map, draft.entry.as_deref(), draft.choice()) {
        Ok(plan) => change_map::start(world, &plan),
        Err(e) => {
            if let Some(mut menu) = world.get_resource_mut::<VehicleMenu>() {
                menu.status = e.to_string();
            }
        }
    }
}
