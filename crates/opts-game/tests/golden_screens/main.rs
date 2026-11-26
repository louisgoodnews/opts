//! Golden screenshots of the screens on **self-authored fixture content** (no OMSI assets or
//! names, ADR-0002), painted by the software painter of `opts_gui::testing::golden` from Bevy's
//! real UI layout and compared with a tolerance.
//!
//! Goldens live in `tests/golden/`. After an intended visual change regenerate them with
//! `UPDATE_GOLDEN=1 cargo test -p opts-game --test golden_screens` and review the diff of the
//! PNGs. A failure writes `<name>.actual.png` and `<name>.diff.png` to `$OPTS_GOLDEN_OUT`
//! (default `tests/golden/_failed/`, git-ignored).
//!
//! The in-game HUD (M-0071: cluster at the bottom left, mini map slot at the bottom right) is
//! painted over a bright and a dark backdrop, with fixture content only. Not covered (the painter
//! is not the GPU): images (so the map's drawing itself, which is tested on pixels in
//! `hud::minimap`), shadows, the focus ring, gradients. The loading screen needs an installation;
//! it is an open item of `docs/src/roadmap/phase-9/accessibility-audit.md`.

#[path = "../common/layout.rs"]
mod layout;
#[path = "../common/screens/mod.rs"]
mod screens;
#[path = "../common/hud/mod.rs"]
mod hud;

use bevy::prelude::*;
use screens::{build, Screen};
use opts_gui::interaction::Activated;
use opts_gui::testing::golden::{check, paint, Tolerance};
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::{AccentPreset, ColorRole, MaterialTheme};
use opts_gui::ui_tree::find_entity;

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

mod determinism;

mod hud_painting;

mod scenes;
