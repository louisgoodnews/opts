//! *Help -> About*: version, licence, third-party components (`docs/src/contributing/legal-licensing.md`).

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

use crate::shell::keys;

/// Third-party components shipped with the game: name and licence (the proper names are not
/// translated).
pub const COMPONENTS: &[(&str, &str)] = &[
    ("Bevy", "MIT or Apache-2.0"),
    (
        "Noto Sans (Noto Project Authors)",
        "SIL Open Font License 1.1",
    ),
    (
        "JetBrains Mono (JetBrains Mono Project Authors)",
        "SIL Open Font License 1.1",
    ),
    (
        "DejaVu Sans (Bitstream, Tavmjong Bah)",
        "Bitstream Vera / DejaVu licence",
    ),
    ("Material Icons (Google)", "Apache-2.0"),
];

fn body(parent: &mut ChildSpawnerCommands, key: &'static str, args: L10nArgs, role: ColorRole) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        args,
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyLarge),
        ForegroundRole(role),
    ));
}

/// Spawns the About content into `parent`.
pub fn spawn_about(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Text::new(keys::BRAND),
        L10nKey(keys::BRAND),
        TextColor::default(),
        TypeStyleToken(TypeScale::Display),
        ForegroundRole(ColorRole::Primary),
    ));
    body(
        parent,
        keys::HELP_ABOUT_VERSION,
        L10nArgs::one("version", env!("CARGO_PKG_VERSION")),
        ColorRole::OnSurface,
    );
    body(
        parent,
        keys::HELP_ABOUT_LICENCE,
        L10nArgs::default(),
        ColorRole::OnSurface,
    );
    body(
        parent,
        keys::HELP_ABOUT_CLEAN,
        L10nArgs::default(),
        ColorRole::OnSurfaceVariant,
    );
    parent.spawn((
        Text::new(keys::HELP_ABOUT_THIRD),
        L10nKey(keys::HELP_ABOUT_THIRD),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
    for (name, licence) in COMPONENTS {
        parent.spawn((
            Text::new(format!("{name} — {licence}")),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
    }
}
