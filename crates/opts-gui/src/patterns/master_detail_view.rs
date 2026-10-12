//! Keeps the master list drawn: rows model, windowed row entities, scroll bar, chips, notice.

use bevy::prelude::*;

use crate::components::button::ButtonBuilder;
use crate::components::text_field::M3TextField;
use crate::icons::icon_text;
use crate::interaction::state::Disabled;
use crate::l10n::{wizard_keys::md, L10nArgs, L10nKey, Localization};
use crate::patterns::glyphs;
use crate::patterns::list_model::{first_item, RowLayout};
use crate::patterns::master_detail::{
    rows_of, MasterChipButton, MasterClearButton, MasterDetail, MasterDetailSelected,
    MasterDetailView, Rendered,
};
use crate::patterns::master_detail_rows::spawn_rows;
use crate::theme::{color::ColorRole, spacing::LayoutClass};
use crate::tokens::components::ForegroundRole;
use crate::ui_id::{slug, UiId};

/// Viewport height assumed while no layout exists yet (headless tests, the first frame).
const FALLBACK_VIEWPORT: f32 = 400.0;

/// Recomputes rows, layout and cursor when the model changed; announces selection changes.
pub(super) fn sync_model(
    mut roots: Query<(Entity, Ref<MasterDetail>, &mut MasterDetailView)>,
    mut selected: MessageWriter<MasterDetailSelected>,
) {
    for (root, model, mut view) in &mut roots {
        if model.is_changed() || view.rendered.is_none() && view.rows.is_empty() {
            view.rows = rows_of(&model);
            view.layout = RowLayout::new(&view.rows);
            view.version += 1;
            let at_selected = model.selected.as_ref().and_then(|id| {
                view.rows
                    .iter()
                    .position(|r| r.entry().is_some_and(|e| &model.entries[e].id == id))
            });
            let keep = view.cursor.filter(|c| {
                view.rows
                    .get(*c)
                    .and_then(|r| r.entry())
                    .is_some_and(|e| model.selected.as_ref() == Some(&model.entries[e].id))
            });
            view.cursor = keep.or(at_selected).or_else(|| first_item(&view.rows));
            let max = view.layout.max_scroll(view.viewport);
            view.scroll = view.scroll.clamp(0.0, max);
        }
        if view.announced.as_ref() != Some(&model.selected) {
            view.announced = Some(model.selected.clone());
            view.reveal_pending = true;
            selected.write(MasterDetailSelected {
                root,
                id: model.selected.clone(),
            });
        }
    }
}

fn viewport_height(computed: &Query<&ComputedNode>, list: Entity) -> f32 {
    computed
        .get(list)
        .map(|c| c.size().y * c.inverse_scale_factor - 2.0)
        .ok()
        .filter(|h| *h > 8.0)
        .unwrap_or(FALLBACK_VIEWPORT)
}

/// Draws the visible window of rows, the scroll bar, the notice, the chips and the placeholder.
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    mut roots: Query<(Entity, &MasterDetail, &mut MasterDetailView)>,
    computed: Query<&ComputedNode>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<(&mut L10nKey, &mut L10nArgs)>,
    loc: Res<Localization>,
    mut text_fields: Query<&mut M3TextField>,
) {
    for (root, model, mut view) in &mut roots {
        let parts = view.parts;
        let height = viewport_height(&computed, parts.list);
        if (height - view.viewport).abs() > 0.5 {
            view.viewport = height;
        }
        let max = view.layout.max_scroll(height);
        view.scroll = view.scroll.clamp(0.0, max);
        if view.reveal_pending && height > 8.0 && !view.layout.is_empty() {
            if let Some(c) = view.cursor {
                view.scroll = view.layout.reveal(c, view.scroll, height).clamp(0.0, max);
            }
            view.reveal_pending = false;
        }
        let range = view.layout.visible_range(view.scroll, height);
        let list_w = computed
            .get(parts.list)
            .map(|c| c.size().x * c.inverse_scale_factor)
            .ok()
            .filter(|w| *w > 40.0)
            .unwrap_or(320.0);
        // Row chrome: marker, thumbnail, star, gaps and padding take about 190 px; a title
        // character is about 9 px wide.
        let chars = (((list_w - 190.0) / 9.0).floor() as usize).max(8);
        let sig = Rendered {
            chars,
            range: range.clone(),
            version: view.version,
            selected: model.selected.clone(),
            favourites: model.favourites.clone(),
        };
        if view.rendered.as_ref() != Some(&sig) {
            for e in std::mem::take(&mut view.row_entities) {
                commands.entity(e).despawn();
            }
            let rows = view.rows.clone();
            let layout = view.layout.clone();
            let prefix = view.config.prefix;
            view.row_entities = spawn_rows(
                &mut commands,
                root,
                prefix,
                model,
                &rows,
                &layout,
                range,
                parts.content,
                chars,
            );
            view.rendered = Some(sig);
        }
        place_content(&mut nodes, &view, parts, height);
        update_notice(&mut nodes, &mut texts, model, &view);
        update_placeholder(&loc, &view, model, &mut text_fields);
        render_chips(&mut commands, root, &mut view, model);
    }
}

fn set_if<T: PartialEq>(slot: &mut T, value: T) {
    if *slot != value {
        *slot = value;
    }
}

fn place_content(
    nodes: &mut Query<&mut Node>,
    view: &MasterDetailView,
    parts: crate::patterns::MasterDetailParts,
    height: f32,
) {
    let total = view.layout.total();
    if let Ok(mut n) = nodes.get_mut(parts.content) {
        set_if(&mut n.height, Val::Px(total));
        set_if(&mut n.top, Val::Px(-view.scroll));
    }
    let scrollable = total > height + 0.5;
    if let Ok(mut n) = nodes.get_mut(parts.track) {
        set_if(
            &mut n.display,
            if scrollable {
                Display::Flex
            } else {
                Display::None
            },
        );
    }
    if scrollable {
        let track = (height - 4.0).max(24.0);
        let thumb = (track * height / total).clamp(24.0, track);
        let max = view.layout.max_scroll(height).max(1.0);
        let top = (track - thumb) * (view.scroll / max);
        if let Ok(mut n) = nodes.get_mut(parts.thumb) {
            set_if(&mut n.height, Val::Px(thumb));
            set_if(&mut n.top, Val::Px(top));
        }
    }
}

fn update_notice(
    nodes: &mut Query<&mut Node>,
    texts: &mut Query<(&mut L10nKey, &mut L10nArgs)>,
    model: &MasterDetail,
    view: &MasterDetailView,
) {
    let parts = view.parts;
    let empty_rows = !view.rows.iter().any(|r| r.is_item());
    if let Ok(mut n) = nodes.get_mut(parts.notice) {
        set_if(
            &mut n.display,
            if empty_rows {
                Display::Flex
            } else {
                Display::None
            },
        );
    }
    if let Ok((mut key, mut args)) = texts.get_mut(parts.notice_text) {
        if model.entries.is_empty() {
            set_if(&mut key.0, view.config.empty_key);
        } else {
            set_if(&mut key.0, md::NO_RESULTS);
            let a = L10nArgs::one("query", model.query.clone()); // l10n-ok: id or variable name
            if *args != a {
                *args = a;
            }
        }
    }
}

fn update_placeholder(
    loc: &Localization,
    view: &MasterDetailView,
    model: &MasterDetail,
    text_fields: &mut Query<&mut M3TextField>,
) {
    let args = [("count".to_string(), model.entries.len().into())]; // l10n-ok: variable name
    let want = loc.t_args(view.config.search_key, &args);
    if let Ok(mut f) = text_fields.get_mut(view.parts.search) {
        if f.placeholder != want {
            f.placeholder = want;
        }
    }
}

fn render_chips(
    commands: &mut Commands,
    root: Entity,
    view: &mut MasterDetailView,
    model: &MasterDetail,
) {
    let sig = (model.chips.clone(), model.active_chips.clone());
    if view.rendered_chips.as_ref() == Some(&sig) {
        return;
    }
    for e in std::mem::take(&mut view.chip_entities) {
        commands.entity(e).despawn();
    }
    let prefix = view.config.prefix;
    let mut spawned = Vec::new();
    for chip in &model.chips {
        let active = model.active_chips.contains(&chip.tag);
        let builder = if active {
            ButtonBuilder::tonal(chip.label.clone())
        } else {
            ButtonBuilder::outlined(chip.label.clone())
        };
        let button = builder.spawn(commands);
        commands.entity(button).insert((
            MasterChipButton {
                root,
                tag: chip.tag.clone(),
            },
            UiId::new(format!("{prefix}.chip.{}", slug(&chip.tag))),
        ));
        if active {
            let mark = commands
                .spawn((
                    icon_text(glyphs::CHECK, 16.0, TextColor::default().0),
                    ForegroundRole(ColorRole::OnSecondaryContainer),
                    crate::ui_tree::A11yHidden,
                ))
                .id();
            commands.entity(button).add_child(mark);
        }
        spawned.push(button);
    }
    commands.entity(view.parts.chips).add_children(&spawned);
    view.chip_entities = spawned;
    view.rendered_chips = Some(sig);
}

/// Stacks the pattern in the Compact layout class and puts the list beside the detail otherwise.
pub(super) fn adapt_layout(
    class: Option<Res<LayoutClass>>,
    roots: Query<&MasterDetailView>,
    mut nodes: Query<&mut Node>,
) {
    let Some(class) = class else { return };
    let compact = *class == LayoutClass::Compact;
    for view in &roots {
        let p = view.parts;
        let root = p.root;
        if let Ok(mut n) = nodes.get_mut(root) {
            set_if(
                &mut n.flex_direction,
                if compact {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
            );
            set_if(
                &mut n.overflow,
                if compact {
                    Overflow::scroll_y()
                } else {
                    Overflow::visible()
                },
            );
        }
        if let Ok(mut n) = nodes.get_mut(p.list_col) {
            set_if(
                &mut n.width,
                if compact {
                    Val::Percent(100.0)
                } else {
                    Val::Percent(40.0)
                },
            );
            set_if(
                &mut n.min_width,
                if compact {
                    Val::Px(0.0)
                } else {
                    Val::Px(320.0)
                },
            );
            set_if(
                &mut n.height,
                if compact { Val::Px(260.0) } else { Val::Auto },
            );
            set_if(&mut n.flex_shrink, if compact { 0.0 } else { 1.0 });
        }
    }
}

/// Disables nothing today; the clear button follows the filter state.
pub(super) fn sync_clear_button(
    roots: Query<(&MasterDetail, &MasterDetailView)>,
    clears: Query<(Entity, &MasterClearButton)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (e, clear) in &clears {
        let Ok((model, _)) = roots.get(clear.0) else {
            continue;
        };
        let filtering = !model.query.trim().is_empty() || !model.active_chips.is_empty();
        if let Ok(mut n) = nodes.get_mut(e) {
            set_if(
                &mut n.display,
                if filtering && !model.entries.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                },
            );
        }
        commands.entity(e).try_remove::<Disabled>();
    }
}

/// Registers the systems.
pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (sync_model, render, adapt_layout, sync_clear_button)
            .chain()
            .after(crate::interaction::activation::ActivationSet)
            .after(super::master_detail_input::InputSet),
    );
}
