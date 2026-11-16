//! Hierarchy construction for the master-detail pattern.

mod list;
mod notice;

use bevy::prelude::*;

use crate::components::text_field::TextFieldBuilder;
use crate::interaction::focus::{FocusRegion, Landmark};
use crate::l10n::{wizard_keys::md, Localization};
use crate::patterns::list_model::RowLayout;
use crate::theme::{color::ColorRole, shape::RadiusLevel};
use crate::tokens::components::{BackgroundRole, RadiusToken};
use crate::ui_id::UiId;

use list::spawn_list_column;

use super::{
    MasterDetail, MasterDetailConfig, MasterDetailParts, MasterDetailRoot, MasterDetailView,
};

pub(super) fn container(direction: FlexDirection) -> Node {
    Node {
        flex_direction: direction,
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..default()
    }
}

fn create_parts(
    commands: &mut Commands,
    loc: &Localization,
    config: &MasterDetailConfig,
    model: &MasterDetail,
) -> (MasterDetailParts, Entity, Entity, Entity) {
    let root = commands.spawn_empty().id();
    let list_col = commands.spawn_empty().id();
    let chips = commands.spawn_empty().id();
    let wrap = commands.spawn_empty().id();
    let list = commands.spawn_empty().id();
    let content = commands.spawn_empty().id();
    let notice = commands.spawn_empty().id();
    let notice_text = commands.spawn_empty().id();
    let track = commands.spawn_empty().id();
    let thumb = commands.spawn_empty().id();
    let detail = commands.spawn_empty().id();
    let panel = commands.spawn_empty().id();
    let args = vec![("count".to_string(), model.entries.len().into())]; // l10n-ok: id or variable name
    let search = TextFieldBuilder::outlined(loc.t_args(md::SEARCH_LABEL, &[]))
        .with_placeholder(loc.t_args(config.search_key, &args))
        .compact()
        .full_width()
        .spawn(commands);
    commands
        .entity(search)
        .insert(UiId::new(format!("{}.search", config.prefix)));
    let parts = MasterDetailParts {
        root,
        detail,
        list,
        search,
        list_col,
        chips,
        content,
        notice,
        notice_text,
        track,
        thumb,
    };
    (parts, root, wrap, panel)
}

fn spawn_detail_panel(commands: &mut Commands, parts: MasterDetailParts, panel: Entity) {
    commands.entity(panel).insert((
        Node {
            flex_grow: 1.0,
            padding: UiRect::all(Val::Px(16.0)),
            overflow: Overflow::scroll_y(),
            ..container(FlexDirection::Column)
        },
        BackgroundRole(ColorRole::SurfaceContainerLow),
        RadiusToken(RadiusLevel::Large),
        FocusRegion(Landmark::Detail),
        Name::new("MasterDetailPanel"),
    ));
    commands.entity(parts.detail).insert((
        Node {
            flex_shrink: 0.0,
            width: Val::Percent(100.0),
            row_gap: Val::Px(12.0),
            ..container(FlexDirection::Column)
        },
        Name::new("MasterDetailContent"),
    ));
    commands.entity(panel).add_child(parts.detail);
    crate::patterns::scroll::wheel_scroll(commands, panel);
}

/// Spawns the pattern (detached: the caller parents `parts.root`) and returns its entities.
pub fn spawn_master_detail(
    commands: &mut Commands,
    loc: &Localization,
    config: MasterDetailConfig,
    model: MasterDetail,
) -> MasterDetailParts {
    let (parts, root, wrap, panel) = create_parts(commands, loc, &config, &model);
    let prefix = config.prefix;
    commands.entity(root).insert((
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            column_gap: Val::Px(16.0),
            row_gap: Val::Px(12.0),
            ..container(FlexDirection::Row)
        },
        MasterDetailRoot,
        model,
        MasterDetailView {
            parts,
            config,
            rows: Vec::new(),
            layout: RowLayout::default(),
            scroll: 0.0,
            cursor: None,
            viewport: 0.0,
            row_entities: Vec::new(),
            rendered: None,
            rendered_chips: None,
            chip_entities: Vec::new(),
            version: 0,
            announced: None,
            reveal_pending: false,
            typed: String::new(),
            typed_at: f32::NEG_INFINITY,
            last_click: None,
        },
        Name::new("MasterDetail"),
    ));
    spawn_list_column(commands, parts, wrap, prefix);
    crate::patterns::master_detail_input::attach_pointer_observers(
        commands,
        root,
        parts.list,
        parts.thumb,
    );
    spawn_detail_panel(commands, parts, panel);
    crate::patterns::scroll::wheel_scroll(commands, root);
    commands.entity(root).add_children(&[parts.list_col, panel]);
    parts
}
