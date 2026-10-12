//! The master-detail pattern: a searchable, virtualised list on the left, a detail panel on the
//! right (stacked in the *Compact* layout class).
//!
//! The screen creates it with [`spawn_master_detail`], keeps the data in the [`MasterDetail`]
//! component of the returned root and fills [`MasterDetailParts::detail`] whenever a
//! [`MasterDetailSelected`] message names a new entry. Everything else — filtering, sections,
//! windowing (at most [`MAX_ROW_ENTITIES`](super::list_model::MAX_ROW_ENTITIES) row entities),
//! keyboard (arrows, paging, Home/End, type-ahead), favourites and the empty states — is the
//! pattern's business. `docs/src/roadmap/phase-9/wizard-and-loading.md` section 2 is the spec.

use bevy::prelude::*;

use crate::components::button::ButtonBuilder;
use crate::components::text_field::TextFieldBuilder;
use crate::interaction::focus::{FocusRegion, Focusable, Landmark, OwnsVerticalArrows};
use crate::interaction::layer::StateLayered;
use crate::l10n::{wizard_keys::md, L10nArgs, L10nKey, Localization};
use crate::patterns::list_model::{build_rows, ListEntry, Row, RowLayout};
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken,
};
use crate::ui_id::UiId;

/// A filter chip: toggles the tag it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    /// The tag an entry must carry.
    pub tag: String,
    /// What the chip says (already localised by the screen).
    pub label: String,
}

/// The data of one master-detail pattern; mutate it and the pattern follows.
#[derive(Component, Debug, Clone, Default)]
pub struct MasterDetail {
    /// All entries, in the order of the *All* section.
    pub entries: Vec<ListEntry>,
    /// Ids of recently used entries, most recent first.
    pub recents: Vec<String>,
    /// Ids of starred entries.
    pub favourites: Vec<String>,
    /// Filter chips offered above the list.
    pub chips: Vec<Chip>,
    /// Tags of the chips that are switched on.
    pub active_chips: Vec<String>,
    /// The search text.
    pub query: String,
    /// Id of the selected entry.
    pub selected: Option<String>,
}

impl MasterDetail {
    /// A model with entries and a selection.
    pub fn new(entries: Vec<ListEntry>, selected: Option<String>) -> Self {
        Self {
            entries,
            selected,
            ..default()
        }
    }

    /// The selected entry.
    pub fn selected_entry(&self) -> Option<&ListEntry> {
        let id = self.selected.as_ref()?;
        self.entries.iter().find(|e| &e.id == id)
    }
}

/// Static settings of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailConfig {
    /// Prefix of the `UiId`s (`wizard.maps`).
    pub prefix: &'static str,
    /// Message key shown instead of the list when the model has no entries at all.
    pub empty_key: &'static str,
    /// Message key of the search placeholder (variable `count`: the number of entries).
    pub search_key: &'static str,
}

/// The entities of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailParts {
    /// Root node: put it where the pattern belongs.
    pub root: Entity,
    /// Container of the detail panel: fill it on [`MasterDetailSelected`].
    pub detail: Entity,
    /// The list (single focus stop).
    pub list: Entity,
    /// The search field (frame).
    pub search: Entity,
    pub(super) list_col: Entity,
    pub(super) chips: Entity,
    pub(super) content: Entity,
    pub(super) notice: Entity,
    pub(super) notice_text: Entity,
    pub(super) track: Entity,
    pub(super) thumb: Entity,
}

/// Internal state of one pattern instance (rows, scroll, what is currently drawn).
#[derive(Component, Debug)]
pub struct MasterDetailView {
    pub(super) parts: MasterDetailParts,
    pub(super) config: MasterDetailConfig,
    pub(super) rows: Vec<Row>,
    pub(super) layout: RowLayout,
    pub(super) scroll: f32,
    pub(super) cursor: Option<usize>,
    pub(super) viewport: f32,
    pub(super) row_entities: Vec<Entity>,
    pub(super) rendered: Option<Rendered>,
    pub(super) rendered_chips: Option<(Vec<Chip>, Vec<String>)>,
    pub(super) chip_entities: Vec<Entity>,
    pub(super) version: u64,
    pub(super) announced: Option<Option<String>>,
    pub(super) reveal_pending: bool,
    pub(super) typed: String,
    pub(super) typed_at: f32,
    pub(super) last_click: Option<(String, f32)>,
}

/// What the row entities currently show (change detection of the renderer).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Rendered {
    pub range: std::ops::Range<usize>,
    pub version: u64,
    pub selected: Option<String>,
    pub favourites: Vec<String>,
}

impl MasterDetailView {
    /// The entities of the pattern.
    pub fn parts(&self) -> MasterDetailParts {
        self.parts
    }

    /// Number of row entities that currently exist (a test and diagnostics hook).
    pub fn live_rows(&self) -> usize {
        self.row_entities.len()
    }

    /// The scroll offset in logical pixels.
    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Total height of all rows.
    pub fn content_height(&self) -> f32 {
        self.layout.total()
    }

    /// The row index of the keyboard position.
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// The flattened rows.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
}

/// The selection changed (also sent once when the pattern first shows a selection).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailSelected {
    /// The pattern root.
    pub root: Entity,
    /// The entry id; `None` when the selection was cleared.
    pub id: Option<String>,
}

/// The user chose the selected entry (Enter, or a second click on it).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailChosen {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// The user starred or un-starred an entry; the pattern has already updated its model, the screen
/// persists the change.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailFavourite {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
    /// The new state.
    pub favourite: bool,
}

/// A thumbnail slot of a row: the screen's thumbnail system fills it (matched by `key`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RowThumb {
    /// The pattern root.
    pub root: Entity,
    /// The thumbnail key of the entry.
    pub key: String,
}

/// Marks the list node (the focus stop) and names its pattern root.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterList(pub Entity);

/// Marks the row button of an entry.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterRowButton {
    /// The pattern root.
    pub root: Entity,
    /// Index into the flattened rows.
    pub row: usize,
    /// The entry id.
    pub id: String,
}

/// Marks the star of a row.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterStarButton {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// Marks a filter chip.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterChipButton {
    /// The pattern root.
    pub root: Entity,
    /// The tag it toggles.
    pub tag: String,
}

/// Marks the *Clear filter* button.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterClearButton(pub Entity);

/// Marks the scroll bar thumb.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterScrollThumb(pub Entity);

/// Marks the root of a pattern.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MasterDetailRoot;

fn container(direction: FlexDirection) -> Node {
    Node {
        flex_direction: direction,
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..default()
    }
}

/// Spawns the pattern (detached: the caller parents `parts.root`) and returns its entities.
pub fn spawn_master_detail(
    commands: &mut Commands,
    loc: &Localization,
    config: MasterDetailConfig,
    model: MasterDetail,
) -> MasterDetailParts {
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
    let prefix = config.prefix;
    commands
        .entity(search)
        .insert(UiId::new(format!("{prefix}.search")));
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
    super::master_detail_input::attach_pointer_observers(commands, root, list, thumb);
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
    // The screen fills this inner node; it never shrinks, so the panel scrolls instead.
    commands.entity(detail).insert((
        Node {
            flex_shrink: 0.0,
            width: Val::Percent(100.0),
            row_gap: Val::Px(12.0),
            ..container(FlexDirection::Column)
        },
        Name::new("MasterDetailContent"),
    ));
    commands.entity(panel).add_child(detail);
    super::scroll::wheel_scroll(commands, panel);
    super::scroll::wheel_scroll(commands, root);
    commands.entity(root).add_children(&[list_col, panel]);
    parts
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
    commands
        .entity(p.list)
        .add_children(&[p.content, p.notice, p.track]);
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

/// Recomputes the flattened rows of a model (used by the systems and by tests).
pub(super) fn rows_of(model: &MasterDetail) -> Vec<Row> {
    build_rows(
        &model.entries,
        &model.recents,
        &model.favourites,
        &model.query,
        &model.active_chips,
    )
}

/// Registers the messages and systems of the pattern.
pub(super) fn build(app: &mut App) {
    app.add_message::<MasterDetailSelected>()
        .add_message::<MasterDetailChosen>()
        .add_message::<MasterDetailFavourite>();
    super::master_detail_input::build(app);
    super::master_detail_view::build(app);
}
