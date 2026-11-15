//! The Interface page's theme line, which follows the theme while the page is open.

use bevy::prelude::*;
use opts_gui::prelude::*;

use super::super::rows::Ctx;
use super::line;

/// The theme line, which follows the theme while the page is open.
pub(super) fn interface_info(c: &mut Commands, pane: Entity, ctx: &Ctx, theme_kind: &str) {
    let now = line(
        c,
        pane,
        "settings-info-theme-now",
        &[("theme", ctx.loc.t(theme_kind))],
        TypeScale::BodyMedium,
    );
    // The theme file list comes from the background facts.
    c.entity(now)
        .insert(crate::content_facts::WantsContentFacts);
    c.queue(|world: &mut World| {
        // The last spawned text of the page is the line above: mark it so it follows
        // the theme while the page is open.
        let mut q = world.query::<(Entity, &opts_gui::l10n::L10nKey)>();
        let found: Vec<Entity> = q
            .iter(world)
            .filter(|(_, k)| k.0 == "settings-info-theme-now")
            .map(|(e, _)| e)
            .collect();
        for e in found {
            world.entity_mut(e).insert(super::super::page::ThemeNowText);
        }
    });
}
