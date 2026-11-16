//! The command table, assembled at compile time from its parts (kept in separate files so that
//! none grows past the file-size limit); the order of the parts is the order of the table.

use super::CommandSpec;

mod basics;
mod config_audio;
mod drive_controls;
mod hud_and_navigation;
mod output_and_capture;
mod reload_and_debug;
mod scene_and_trace;
mod session_saves;
mod settings_and_l10n;
mod sim_and_scene;
mod ui_screen;
mod vehicle_and_spawn;
mod vehicle_dynamics;
mod viewpoint;

const PARTS: &[&[CommandSpec]] = &[
    basics::PART,
    sim_and_scene::PART,
    settings_and_l10n::PART,
    reload_and_debug::PART,
    output_and_capture::PART,
    ui_screen::PART,
    drive_controls::PART,
    scene_and_trace::PART,
    hud_and_navigation::PART,
    vehicle_and_spawn::PART,
    viewpoint::PART,
    session_saves::PART,
    vehicle_dynamics::PART,
    config_audio::PART,
];

/// The number of entries of all parts.
pub(super) const LEN: usize = {
    let mut n = 0;
    let mut i = 0;
    while i < PARTS.len() {
        n += PARTS[i].len();
        i += 1;
    }
    n
};

/// All parts one after another, in order.
pub(super) const fn join() -> [CommandSpec; LEN] {
    let mut out = [PARTS[0][0]; LEN];
    let mut k = 0;
    let mut i = 0;
    while i < PARTS.len() {
        let mut j = 0;
        while j < PARTS[i].len() {
            out[k] = PARTS[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    out
}
