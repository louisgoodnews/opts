//! The texts of the fuel widgets (M-0123): the level in percent, and the low-level lamp, which
//! reads `LOW` in the alarm colour at or below the threshold (a word, not only a colour).

use opts_gui::l10n::Localization;

use super::super::fuel_hud::FuelHud;
use super::super::keys;
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
