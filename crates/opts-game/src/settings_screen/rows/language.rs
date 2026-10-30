//! The language list of the settings rows: installed languages and their completeness.

use super::*;

/// The notes under a language pack: how much of it is translated, by whom, and its gaps, age and
/// problems.
fn pack_notes(c: &mut Commands, col: Entity, l: &opts_gui::l10n::LanguageInfo) {
    let percent = l.percent().round() as usize;
    if l.authors.is_empty() {
        note(
            c,
            col,
            "set-lang-info-unsigned",
            vec![("percent".into(), percent.into())],
            ColorRole::OnSurfaceVariant,
        );
    } else {
        note(
            c,
            col,
            "set-lang-info",
            vec![
                ("percent".into(), percent.into()),
                ("authors".into(), l.authors.join(", ").into()),
            ],
            ColorRole::OnSurfaceVariant,
        );
    }
    let gaps = l.missing + l.copied;
    if gaps > 0 {
        note(
            c,
            col,
            "set-lang-gaps",
            vec![("count".into(), gaps.into())],
            ColorRole::OnSurfaceVariant,
        );
    }
    if l.outdated {
        note(
            c,
            col,
            "set-lang-outdated",
            Vec::new(),
            ColorRole::OnSurfaceVariant,
        );
    }
    if l.errors > 0 {
        note(
            c,
            col,
            "set-lang-problems",
            vec![("count".into(), l.errors.into())],
            ColorRole::Error,
        );
    }
}

/// The languages of the interface: one button each (named in its own language), and below a
/// pack's button how complete it is, who made it and what is wrong with it. Right-to-left packs
/// are shown but cannot be chosen.
pub(super) fn language_list(
    c: &mut Commands,
    group: Entity,
    ctx: &Ctx,
    d: &SettingDescriptor,
    cur: &str,
    u: &Usable,
) {
    for l in ctx.loc.languages() {
        let col = c
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                ChildOf(group),
            ))
            .id();
        let b = button(
            c,
            col,
            d,
            format!("option-{}", l.code),
            None,
            &l.native_name,
            l.code == cur,
            SettingOp::Set {
                key: d.key,
                value: l.code.clone(),
            },
            u,
        );
        if !l.selectable {
            c.entity(b)
                .insert((Disabled, DisabledReason(ctx.loc.t("set-lang-rtl").into())));
            note(c, col, "set-lang-rtl", Vec::new(), ColorRole::Error);
        }
        if l.from_pack {
            pack_notes(c, col, &l);
        }
        if !l.missing_fonts.is_empty() {
            note(
                c,
                col,
                "set-lang-fonts",
                vec![("scripts".into(), l.missing_fonts.join(", ").into())],
                ColorRole::Error,
            );
        }
    }
    if let Some(dir) = crate::language_packs::packs_dir() {
        let e = c
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
                ChildOf(group),
            ))
            .id();
        note(
            c,
            e,
            "set-lang-folder",
            vec![("path".into(), dir.display().to_string().into())],
            ColorRole::OnSurfaceVariant,
        );
    }
}
