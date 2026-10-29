//! Hierarchy construction for the master-detail pattern.

use bevy::prelude::*;

use crate::components::button::ButtonBuilder;
use crate::components::text_field::TextFieldBuilder;
use crate::interaction::focus::{FocusRegion, Focusable, Landmark, OwnsVerticalArrows};
use crate::interaction::layer::StateLayered;
use crate::l10n::{wizard_keys::md, L10nArgs, L10nKey, Localization};
use crate::patterns::list_model::RowLayout;
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken,
};
use crate::ui_id::UiId;

use super::{
    MasterClearButton, MasterDetail, MasterDetailConfig, MasterDetailParts, MasterDetailRoot,
    MasterDetailView, MasterList, MasterScrollThumb,
};

fn container(direction: FlexDirection) -> Node {
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

fn spawn_track_and_thumb(commands: &mut Commands, p: MasterDetailParts) {
    commands.entity(p.track).insert((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(2.0),
            bottom: Val::Px(2.0),
            width: Val::Px(8.0),
            display: Display::None,
            ..default()
        },
        Name::new("MasterScrollTrack"),
    ));
    commands.entity(p.thumb).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(24.0),
            ..default()
        },
        BackgroundRole(ColorRole::Outline),
        RadiusToken(RadiusLevel::Full),
        MasterScrollThumb(p.root),
        Name::new("MasterScrollThumb"),
    ));
    commands.entity(p.track).add_child(p.thumb);
}

fn spawn_viewport(commands: &mut Commands, p: MasterDetailParts, prefix: &str) {
    commands.entity(p.list).insert((
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            overflow: Overflow::clip_y(),
            border: UiRect::all(Val::Px(1.0)),
            ..container(FlexDirection::Column)
        },
        StateLayered,
        Focusable::default(),
        OwnsVerticalArrows,
        MasterList(p.root),
        BackgroundRole(ColorRole::SurfaceContainerLow),
        BorderRole(ColorRole::OutlineVariant),
        RadiusToken(RadiusLevel::Medium),
        Name::new("MasterListViewport"),
    ));
    commands.entity(p.content).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(0.0),
            ..default()
        },
        Name::new("MasterListContent"),
    ));
    spawn_notice(commands, p, prefix);
    commands
        .entity(p.list)
        .insert(UiId::new(format!("{prefix}.list")));
    spawn_track_and_thumb(commands, p);
    commands
        .entity(p.list)
        .add_children(&[p.content, p.notice, p.track]);
}

fn spawn_list_column(commands: &mut Commands, p: MasterDetailParts, wrap: Entity, prefix: &str) {
    commands.entity(p.list_col).insert((
        Node {
            width: Val::Percent(40.0),
            min_width: Val::Px(320.0),
            max_width: Val::Px(520.0),
            row_gap: Val::Px(8.0),
            ..container(FlexDirection::Column)
        },
        FocusRegion(Landmark::Content),
        Name::new("MasterList"),
    ));
    commands.entity(p.chips).insert((
        Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..container(FlexDirection::Row)
        },
        Name::new("MasterChips"),
    ));
    commands.entity(wrap).insert((
        Node {
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            ..container(FlexDirection::Column)
        },
        Name::new("MasterListWrap"),
    ));
    spawn_viewport(commands, p, prefix);
    commands.entity(wrap).add_child(p.list);
    commands
        .entity(p.list_col)
        .add_children(&[p.search, p.chips, wrap]);
}

fn spawn_notice(commands: &mut Commands, p: MasterDetailParts, prefix: &str) {
    commands.entity(p.notice).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..container(FlexDirection::Column)
        },
        Name::new("MasterNotice"),
    ));
    commands.entity(p.notice_text).insert((
        Text::new(md::NO_RESULTS),
        L10nKey(md::NO_RESULTS),
        L10nArgs::one("query", ""), // l10n-ok: id or variable name
        TextColor::default(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        TypeStyleToken(TypeScale::BodyMedium),
    ));
    let clear = ButtonBuilder::outlined(md::CLEAR_FILTER)
        .with_key(md::CLEAR_FILTER)
        .spawn(commands);
    commands.entity(clear).insert((
        MasterClearButton(p.root),
        UiId::new(format!("{prefix}.clear")),
    ));
    commands
        .entity(p.notice)
        .add_children(&[p.notice_text, clear]);
}
