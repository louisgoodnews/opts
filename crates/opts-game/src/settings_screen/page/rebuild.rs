//! Rebuilding the settings page: the category list, the notice and the lighting section.

use super::*;

/// Fills the category list and the pane.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_page(
    mut commands: Commands,
    mut state: ResMut<SettingsPageState>,
    theme: Res<MaterialTheme>,
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    caps: Res<SettingsCapabilities>,
    facts: Res<crate::content_facts::ContentFactsStore>,
    loc: Res<Localization>,
    hardware: Option<Res<crate::hardware::HardwareState>>,
    light: Option<Res<opts_bevy::light_quality::LightQuality>>,
    fonts: Option<Res<crate::ui_typography::FontChoices>>,
    focused: Query<&UiId, With<opts_gui::interaction::state::Focused>>,
    cats: Query<(Entity, Option<&Children>), With<PageCategories>>,
    panes: Query<(Entity, Option<&Children>), With<PagePane>>,
) {
    let (Ok((cat_list, cat_kids)), Ok((pane, pane_kids))) = (cats.single(), panes.single()) else {
        return;
    };
    if !state.dirty {
        return;
    }
    state.dirty = false;
    if let Some(id) = focused.iter().map(|u| u.0.to_string()).find(|id| {
        id.starts_with("settings/")
            && !id.starts_with("settings/category/")
            && id != "settings/search"
            && id != "settings/back"
    }) {
        state.restore_focus = Some(id);
    }
    for k in pane_kids.into_iter().flatten() {
        commands.entity(*k).despawn();
    }
    // The category buttons stay (focus and activation survive a category change); only their
    // look follows the state (`style_categories`).
    let build_categories = cat_kids.is_none_or(|k| k.is_empty());
    let ctx = Ctx {
        registry: &registry,
        settings: &settings,
        loc: &loc,
        caps: &caps,
        hardware: hardware.as_ref().and_then(|h| h.profile.as_ref()),
        hardware_file: hardware
            .as_ref()
            .filter(|h| h.is_stored())
            .and_then(|h| h.dir.as_deref().map(opts_hardware::store::path)),
        theme_files: facts.theme_files(),
        fonts: fonts.as_ref().map_or(&[][..], |f| f.0.as_slice()),
        light: light.as_deref(),
        rebinding: state.rebinding,
    };
    let notice = state.notice.clone();
    let searching = !state.query.trim().is_empty();
    for cat in registry
        .categories()
        .into_iter()
        .filter(|_| build_categories)
    {
        let visible = registry.in_category(cat).iter().any(|d| !d.hidden);
        if !visible {
            continue;
        }
        let b = ButtonBuilder::text(cat.label_key())
            .with_key(cat.label_key())
            .spawn(&mut commands);
        commands.entity(b).insert((
            SettingOp::Category(cat),
            UiId::new(format!("settings/category/{}", cat.id())),
            ChildOf(cat_list),
        ));
    }
    if searching {
        spawn_notice(&mut commands, pane, notice.as_deref());
        let found = search::search(&registry, &loc, &state.query);
        if found.is_empty() {
            label(
                &mut commands,
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
                    &mut commands,
                    pane,
                    d.category.label_key(),
                    TypeScale::TitleLarge,
                    ColorRole::Primary,
                );
            }
            light_section(&mut commands, pane, &ctx, d);
            spawn_row(&mut commands, pane, &ctx, d);
        }
    } else {
        label(
            &mut commands,
            pane,
            state.category.label_key(),
            TypeScale::TitleLarge,
            ColorRole::Primary,
        );
        spawn_notice(&mut commands, pane, notice.as_deref());
        for d in registry
            .in_category(state.category)
            .into_iter()
            .filter(|d| !d.hidden)
        {
            light_section(&mut commands, pane, &ctx, d);
            spawn_row(&mut commands, pane, &ctx, d);
            if d.key == "content.rescan" {
                super::super::scan_status::spawn_chip(&mut commands, pane);
            }
        }
        let kind = theme_name_key(theme.kind());
        super::super::info::spawn_info(&mut commands, pane, &ctx, state.category, kind);
    }
}

/// The line above the rows (see [`SettingsPageState::notice`]).
fn spawn_notice(commands: &mut Commands, pane: Entity, notice: Option<&str>) {
    let Some(text) = notice else { return };
    commands.spawn((
        Text::new(text),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyLarge),
        ForegroundRole(ColorRole::Error),
        UiId::new("settings/notice"),
        ChildOf(pane),
    ));
}

/// The "Lighting quality" section heading and its note, above the mode row.
fn light_section(
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
