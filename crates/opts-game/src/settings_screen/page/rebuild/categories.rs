//! The category list and the category pages: buttons, search results and one category's rows.

use super::*;

/// The category buttons that have a visible setting.
pub(super) fn spawn_category_buttons(
    commands: &mut Commands,
    cat_list: Entity,
    registry: &SettingsRegistry,
) {
    for cat in registry.categories() {
        let visible = registry.in_category(cat).iter().any(|d| !d.hidden);
        if !visible {
            continue;
        }
        let b = ButtonBuilder::text(cat.label_key())
            .with_key(cat.label_key())
            .spawn(commands);
        commands.entity(b).insert((
            SettingOp::Category(cat),
            UiId::new(format!("settings/category/{}", cat.id())),
            ChildOf(cat_list),
        ));
    }
}

/// The page of a search: the settings that match, under the headline of their category.
pub(super) fn spawn_search_results(
    commands: &mut Commands,
    pane: Entity,
    ctx: &Ctx,
    (query, notice): (&str, Option<&str>),
) {
    spawn_notice(commands, pane, notice);
    let found = search::search(ctx.registry, ctx.loc, query);
    if found.is_empty() {
        label(
            commands,
            pane,
            "settings-no-results",
            TypeScale::BodyLarge,
            ColorRole::OnSurfaceVariant,
        );
    }
    let mut last = None;
    for d in found.into_iter().filter(|d| !d.hidden) {
        if last != Some(d.category) {
            last = Some(d.category);
            label(
                commands,
                pane,
                d.category.label_key(),
                TypeScale::TitleLarge,
                ColorRole::Primary,
            );
        }
        light_section(commands, pane, ctx, d);
        spawn_row(commands, pane, ctx, d);
    }
}

/// The page of one category: its headline, notice, rows and info lines.
pub(super) fn spawn_category(
    commands: &mut Commands,
    pane: Entity,
    ctx: &Ctx,
    (category, notice, theme_kind): (Category, Option<&str>, &str),
) {
    label(
        commands,
        pane,
        category.label_key(),
        TypeScale::TitleLarge,
        ColorRole::Primary,
    );
    spawn_notice(commands, pane, notice);
    for d in ctx
        .registry
        .in_category(category)
        .into_iter()
        .filter(|d| !d.hidden)
    {
        light_section(commands, pane, ctx, d);
        spawn_row(commands, pane, ctx, d);
        if d.key == "content.rescan" {
            super::super::super::scan_status::spawn_chip(commands, pane);
        }
    }
    super::super::super::info::spawn_info(commands, pane, ctx, category, theme_kind);
}
