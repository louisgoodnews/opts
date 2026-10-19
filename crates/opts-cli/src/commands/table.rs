//! The command table, assembled at compile time from its parts (kept in separate files so that
//! none grows past the file-size limit); the order of the parts is the order of the table.

use super::CommandSpec;

mod basics;
mod drive_and_session;
mod scene_and_trace;
mod ui_and_capture;

const PARTS: &[&[CommandSpec]] = &[
    basics::PART,
    ui_and_capture::PART,
    scene_and_trace::PART,
    drive_and_session::PART,
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
