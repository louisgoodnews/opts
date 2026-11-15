//! The lighting quality section of the page (its heading, note and the ladder level names).

use super::*;

/// The "Lighting quality" section heading and its note, above the mode row.
pub(super) fn light_section(
    commands: &mut Commands,
    pane: Entity,
    ctx: &Ctx,
    d: &crate::settings_registry::SettingDescriptor,
) {
    use crate::settings_registry::lighting as l;
    if d.key != l::MODE_KEY {
        return;
    }
    let heading = label(
        commands,
        pane,
        "set-light-section",
        TypeScale::TitleMedium,
        ColorRole::Primary,
    );
    commands.entity(heading).insert(Node {
        margin: UiRect::top(Val::Px(20.0)),
        ..default()
    });
    label(
        commands,
        pane,
        "set-light-section-note",
        TypeScale::BodyMedium,
        ColorRole::OnSurfaceVariant,
    );
    if l::is_automatic(ctx.settings) {
        let level = ctx.light.map_or(0, |q| q.level);
        commands.spawn((
            Text::new("set-light-auto-note"),
            opts_gui::l10n::L10nKey("set-light-auto-note"),
            L10nArgs(vec![
                ("level".into(), usize::from(level).into()),
                ("name".into(), ctx.loc.t(level_key(level)).into()),
            ]),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::Info),
            ChildOf(pane),
        ));
    }
}

/// The message key of the name of a ladder level.
#[must_use]
pub fn level_key(level: u8) -> &'static str {
    match level {
        0 => "set-light-level-0",
        1 => "set-light-level-1",
        2 => "set-light-level-2",
        3 => "set-light-level-3",
        4 => "set-light-level-4",
        _ => "set-light-level-5",
    }
}
