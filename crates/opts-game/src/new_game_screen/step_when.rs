//! Step ③ When: quick times, fixed-size time steppers, weather presets. The date steppers (built
//! here with [`date_block`]) are on the map step, the one place where the date is chosen.

mod cards;
mod quick;
mod refresh;
mod time;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::BuildCtx;
use crate::new_game_screen::planner::{Field, TimeSource};
use crate::new_game_screen::ui::{Act, PlannerAction, column, keyed};
use crate::shell::calendar;

use cards::weather_cards;
use quick::quick_chips;
use time::time_block;

pub use refresh::refresh_when;
pub use time::date_block;

/// A value text of a stepper; [`refresh_when`] keeps it current.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhenValue(pub Field);

/// The era hint line (shown only outside the map's years).
#[derive(Component, Debug, Clone, Copy)]
pub struct EraHint;

/// A line under the time that says where a default time comes from; [`refresh_when`] shows the
/// one that matches [`PlannerState::time_source`] and none once the user has set the time.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSourceLabel(pub TimeSource);

/// One stepper of the plan (`shell::calendar`): ids `wizard.<step>.<field>.minus|plus`.
pub fn stepper(commands: &mut Commands, field: Field, value: String, id_prefix: &str) -> Entity {
    calendar::spawn_stepper(commands, field, value, WhenValue(field), id_prefix, |d| {
        Act(PlannerAction::Step(field, d))
    })
}

fn section_title(commands: &mut Commands, key: &'static str) -> Entity {
    commands
        .spawn(keyed(key, TypeScale::TitleMedium, ColorRole::OnSurface))
        .id()
}

/// Builds the step body.
pub fn spawn_when(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                padding: UiRect::all(Val::Px(4.0)),
                overflow: Overflow::scroll_y(),
                ..column(16.0)
            },
            Name::new("WhenStep"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, body);
    let quick_title = section_title(commands, "wiz-when-quick");
    let quick = quick_chips(commands);
    let dt_title = section_title(commands, "wiz-when-exact");
    let dt = time_block(commands, ctx);
    let w_title = section_title(commands, "wiz-when-weather");
    let weather = weather_cards(commands, ctx);
    commands
        .entity(body)
        .add_children(&[quick_title, quick, dt_title, dt, w_title, weather]);
    super::ui::no_shrink(
        commands,
        &[quick_title, quick, dt_title, dt, w_title, weather],
    );
    body
}
