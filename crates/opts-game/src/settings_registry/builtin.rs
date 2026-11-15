//! The settings the game itself registers: the keys of earlier versions (their stored names and
//! values are unchanged) and the new ones of M-0063.

mod advanced;
mod enums;
mod files;
mod hud;
mod profile;

use opts_gui::not_yet::{SETTINGS_REBIND, SETTINGS_TIME_SCALE};

use super::descriptor::{Apply, Category as C, SettingDescriptor as D, SettingKind as K};

use advanced::{advanced, streaming};
use enums::*;
use files::files;
use hud::{hud, minimap};
use profile::MAP_PROFILE_KEYS;

mod graphics;
use graphics::graphics;

mod interface;
use interface::interface;

/// Part 3: content and advanced.
fn content() -> Vec<D> {
    [files(), advanced(), streaming()].concat()
}

/// Every built-in descriptor.
#[must_use]
pub fn all() -> Vec<D> {
    let mut v = all_unscoped();
    for d in &mut v {
        if MAP_PROFILE_KEYS.contains(&d.key) {
            d.profile = super::descriptor::ProfileScope::Map;
        }
    }
    v
}

fn all_unscoped() -> Vec<D> {
    let mut v = graphics();
    v.extend(super::lighting::descriptors());
    v.extend(interface());
    v.extend(hud());
    v.extend(minimap());
    v.extend(content());
    v.extend(crate::hardware::descriptors());
    v.extend(crate::vehicle_keys::descriptors());
    v.extend(crate::action_settings::descriptors());
    v.extend(crate::vehicle_setup_settings::descriptors());
    v
}
