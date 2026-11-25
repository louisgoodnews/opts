//! Message keys of the shell (the tables are `l10n/shell-en.ftl` and `l10n/shell-de.ftl`).
//!
//! Key tables may hold string literals; everything else in the shell may not (see the test
//! `no_user_visible_literals_in_the_shell`). The keys sit in the group modules below; [`ALL`]
//! joins their lists.

macro_rules! keys {
    ($($(#[$m:meta])* $name:ident = $text:literal),* $(,)?) => {
        $($(#[$m])* pub const $name: &str = $text;)*
        /// Every key above; the completeness tests iterate it.
        pub const ALL: &[&str] = &[$($name),*];
    };
}

mod chrome;
mod firstrun;
mod help;
mod home;
mod overlays;
mod saves;

pub use chrome::*;
pub use firstrun::*;
pub use help::*;
pub use home::*;
pub use overlays::*;
pub use saves::*;

const PARTS: &[&[&str]] = &[
    chrome::ALL,
    firstrun::ALL,
    help::ALL,
    home::ALL,
    overlays::ALL,
    saves::ALL,
];

const fn total(parts: &[&[&str]]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

const fn join<const N: usize>(parts: &[&[&'static str]]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut k = 0;
    let mut i = 0;
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    out
}

/// Every key of the groups; the completeness tests iterate it.
pub const ALL: &[&str] = &join::<{ total(PARTS) }>(PARTS);

/// The key of the rail label of a registered screen id, if the shell knows one.
pub fn nav_title(id: &str) -> Option<&'static str> {
    Some(match id {
        "home" => NAV_HOME,
        "drive" => NAV_DRIVE,
        "library" => NAV_LIBRARY,
        "workshop" => NAV_WORKSHOP,
        "settings" => NAV_SETTINGS,
        "help" => NAV_HELP,
        _ => return None,
    })
}
