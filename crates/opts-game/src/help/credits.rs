//! *Help -> Credits* (ISS-082): the project's credits and the third-party components. The
//! project text is the copyright line of `LICENSE-MIT` and the licence wording of
//! `docs/src/contributing/legal-licensing.md`; the component list is the one of the About tab
//! ([`super::about::COMPONENTS`]), so the two cannot drift apart.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::shell::keys;

use super::about::COMPONENTS;

/// The third-party lines, `name — licence`, in the order of [`COMPONENTS`].
pub fn component_lines() -> Vec<String> {
    COMPONENTS
        .iter()
        .map(|(name, licence)| format!("{name} — {licence}"))
        .collect()
}

fn body(parent: &mut ChildSpawnerCommands, key: &'static str, role: ColorRole) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyLarge),
        ForegroundRole(role),
    ));
}

/// Spawns the Credits content into `parent`.
pub(super) fn spawn_credits(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Text::new(keys::HELP_CREDITS_TITLE),
        L10nKey(keys::HELP_CREDITS_TITLE),
        TextColor::default(),
        TypeStyleToken(TypeScale::Display),
        ForegroundRole(ColorRole::Primary),
    ));
    body(parent, keys::HELP_CREDITS_LICENCE, ColorRole::OnSurface);
    body(parent, keys::HELP_CREDITS_SCOPE, ColorRole::OnSurfaceVariant);
    body(
        parent,
        keys::HELP_CREDITS_LICENCE_FILES,
        ColorRole::OnSurfaceVariant,
    );
    parent.spawn((
        Text::new(keys::HELP_ABOUT_THIRD),
        L10nKey(keys::HELP_ABOUT_THIRD),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
    for line in component_lines() {
        parent.spawn((
            Text::new(line),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_third_party_component_has_one_line_with_its_licence() {
        let lines = component_lines();
        assert_eq!(lines.len(), COMPONENTS.len());
        for ((name, licence), line) in COMPONENTS.iter().zip(&lines) {
            assert!(line.starts_with(name), "{line}");
            assert!(line.ends_with(licence), "{line}");
        }
    }

    #[test]
    fn the_credits_tab_is_last_and_reachable_by_its_id() {
        assert_eq!(HelpTab::ALL.last(), Some(&HelpTab::Credits));
        assert_eq!(HelpTab::parse("credits"), Some(HelpTab::Credits));
    }
}
