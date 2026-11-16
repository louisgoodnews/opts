//! Keeps the master list drawn: rows model, windowed row entities, scroll bar, chips, notice.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::components::text_field::M3TextField;
use crate::l10n::{L10nArgs, L10nKey, Localization};
use crate::patterns::list_model::{first_item, RowLayout};
use crate::patterns::master_detail::{
    rows_of, MasterDetail, MasterDetailSelected, MasterDetailView, Rendered,
};
use crate::patterns::master_detail_rows::{spawn_rows, SpawnRowsContext};

mod chips;
mod chrome;
mod notice;

use chips::{render_chips, sync_clear_button};
use chrome::{adapt_layout, place_content};
use notice::{update_notice, update_placeholder};

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

/// Parameters for [`render`].
#[derive(SystemParam)]
pub(super) struct MasterDetailRenderParams<'w, 's> {
    pub roots: Query<'w, 's, (Entity, &'static MasterDetail, &'static mut MasterDetailView)>,
    pub computed: Query<'w, 's, &'static ComputedNode>,
    pub nodes: Query<'w, 's, &'static mut Node>,
    pub texts: Query<'w, 's, (&'static mut L10nKey, &'static mut L10nArgs)>,
    pub loc: Res<'w, Localization>,
    pub text_fields: Query<'w, 's, &'static mut M3TextField>,
}

/// Draws the visible window of rows, the scroll bar, the notice, the chips and the placeholder.
pub(super) fn render(mut commands: Commands, mut params: MasterDetailRenderParams) {
    for (root, model, mut view) in &mut params.roots {
        let parts = view.parts;
        let height = viewport_height(&params.computed, parts.list);
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
        let list_w = params
            .computed
            .get(parts.list)
            .map(|c| c.size().x * c.inverse_scale_factor)
            .ok()
            .filter(|w| *w > 40.0)
            .unwrap_or(320.0);
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
            let ctx = SpawnRowsContext {
                root,
                prefix,
                model,
                rows: &rows,
                layout: &layout,
                content: parts.content,
                max_chars: chars,
            };
            view.row_entities = spawn_rows(&mut commands, &ctx, range);
            view.rendered = Some(sig);
        }
        place_content(&mut params.nodes, &view, parts, height);
        update_notice(&mut params.nodes, &mut params.texts, model, &view);
        update_placeholder(&params.loc, &view, model, &mut params.text_fields);
        render_chips(&mut commands, root, &mut view, model);
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
