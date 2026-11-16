//! Rebuilding the settings page: the category list, the notice and the lighting section.

use super::*;

/// Fills the category list and the pane.
pub fn rebuild_page(
    mut commands: Commands,
    mut state: ResMut<SettingsPageState>,
    inputs: super::params::RebuildFacts,
    focused: Query<&UiId, With<opts_gui::interaction::state::Focused>>,
    cats: Query<(Entity, Option<&Children>), With<PageCategories>>,
    panes: Query<(Entity, Option<&Children>), With<PagePane>>,
) {
    let super::params::RebuildFacts {
        theme,
        registry,
        settings,
        caps,
        facts,
        loc,
        hardware,
        light,
        fonts,
    } = inputs;
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
    if build_categories {
        spawn_category_buttons(&mut commands, cat_list, &registry);
    }
    if searching {
        spawn_search_results(&mut commands, pane, &ctx, (&state.query, notice.as_deref()));
    } else {
        let kind = theme_name_key(theme.kind());
        spawn_category(
            &mut commands,
            pane,
            &ctx,
            (state.category, notice.as_deref(), kind),
        );
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
mod categories;
mod light;
use categories::{spawn_category, spawn_category_buttons, spawn_search_results};
pub use light::level_key;
use light::light_section;
