//! The texts of the fuel widgets (M-0123): the level in percent, and the low-level lamp, which
//! reads `LOW` in the alarm colour at or below the threshold (a word, not only a colour).

use bevy::prelude::*;
use opts_gui::l10n::Localization;
use opts_gui::theme::{InstrumentVariant, MaterialTheme};

use super::super::build::{HudBarFill, HudBarTrack};
use super::super::fuel_hud::FuelHud;
use super::super::keys;
use super::super::values::HudValues;
use crate::in_game::instruments::InstrumentInk;

/// The level in percent; empty without a readout (no model, or consumption `Off`).
pub(super) fn level_text(fuel: Option<FuelHud>) -> String {
    fuel.map_or_else(String::new, |f| format!("{:.0} %", f.percent))
}

/// The low-level lamp text, with its ink when the lamp has one.
pub(super) fn lamp_text(
    loc: &Localization,
    fuel: Option<FuelHud>,
    ink: Option<&mut InstrumentInk>,
) -> String {
    let low = fuel.is_some_and(|f| f.low);
    if let Some(ink) = ink {
        let want = if low {
            InstrumentInk::Alarm
        } else {
            InstrumentInk::Ok
        };
        if *ink != want {
            *ink = want;
        }
    }
    loc.t(if low { keys::FUEL_LOW } else { keys::FUEL_OK })
}

/// The fill width of the fuel bar in percent of its track (0 without a readout).
#[must_use]
pub(super) fn bar_percent(fuel: Option<FuelHud>) -> f32 {
    fuel.map_or(0.0, |f| f.percent.clamp(0.0, 100.0))
}

/// Sets the fuel bar's fill width and colours: the fill takes the alarm colour at the low level,
/// the track the palette's panel border. The percentage is also in the readout text.
pub fn update_fuel_bars(
    values: Res<HudValues>,
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    mut fills: Query<(&mut Node, &mut BackgroundColor), With<HudBarFill>>,
    mut tracks: Query<&mut BackgroundColor, (With<HudBarTrack>, Without<HudBarFill>)>,
) {
    let palette = theme.instrument(*variant);
    let low = values.fuel.is_some_and(|f| f.low);
    let fill = if low { palette.alarm } else { palette.ok };
    let width = Val::Percent(bar_percent(values.fuel));
    for (mut node, mut color) in &mut fills {
        if node.width != width {
            node.width = width;
        }
        if color.0 != fill {
            color.0 = fill;
        }
    }
    for mut color in &mut tracks {
        if color.0 != palette.panel_border {
            color.0 = palette.panel_border;
        }
    }
}

#[cfg(test)]
mod bar_tests {
    use super::*;

    #[test]
    fn the_bar_follows_the_level_and_is_empty_without_a_readout() {
        assert_eq!(bar_percent(None), 0.0);
        let half = FuelHud {
            percent: 50.0,
            low: false,
        };
        assert_eq!(bar_percent(Some(half)), 50.0);
        let over = FuelHud {
            percent: 140.0,
            low: false,
        };
        assert_eq!(bar_percent(Some(over)), 100.0);
    }
}
