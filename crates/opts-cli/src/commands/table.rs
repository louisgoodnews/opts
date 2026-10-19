//! The command table, assembled at compile time from its parts (kept in separate files so that
//! none grows past the file-size limit).

use super::CommandSpec;

mod part_1;
mod part_2;
mod part_3;
mod part_4;

const PARTS: &[&[CommandSpec]] = &[part_1::PART, part_2::PART, part_3::PART, part_4::PART];

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
