//! M-0065: the automatic accessibility rules over every shell screen, in every theme and accent,
//! at three window sizes, and in the pseudo-locale and German (text expansion).

mod common;

use bevy::prelude::*;
use common::screens::{build, set_locale, set_theme, Screen};
use opts_gui::testing::rules;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::{AccentPreset, MaterialTheme};

const KINDS: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

#[path = "a11y_matrix/contrast.rs"]
mod contrast;
#[path = "a11y_matrix/controls.rs"]
mod controls;
#[path = "a11y_matrix/scale.rs"]
mod scale;
