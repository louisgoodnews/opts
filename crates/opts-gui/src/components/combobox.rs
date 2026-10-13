//! Material Design 3 Combobox / Dropdown selection widget.

use crate::components::text_field::{M3TextField, TextFieldBuilder, TextFieldInput};
use crate::components::Button;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;

use bevy::input_focus::InputFocusVisible;

use crate::l10n::Localization;

use crate::{
    actions::{Direction, UiAction},
    interaction::{
        activation::Activated,
        events::ComboboxSelected,
        focus::{FocusEvent, OwnsVerticalArrows},
        layer::FocusProxy,
        state::{Disabled, Focused},
    },
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

/// Marker component for an M3 Combobox root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Combobox {
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub placeholder: String,
    pub is_open: bool,
    /// Menus with more options than this have a search field (fixed when the menu is spawned).
    pub search_threshold: usize,
}

impl M3Combobox {
    /// Whether the menu has a search field.
    pub fn searchable(&self) -> bool {
        self.options.len() > self.search_threshold
    }
}

/// Menus with more options than this get a search field (see [`ComboboxBuilder::with_search_threshold`]).
pub const DEFAULT_SEARCH_THRESHOLD: usize = 8;

/// How long the filter waits after the last keystroke.
pub const FILTER_DEBOUNCE_SECS: f32 = 0.15;

/// The search field of a long menu (on the text field frame); points at the combobox root.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxSearch(pub Entity);

/// The scrolling list of options inside the menu (the search field stays above it).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxList(pub Entity);

/// The "No match" row of a searchable menu.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxNoMatch(pub Entity);

/// Search state of a searchable combobox (on the root).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ComboboxFilter {
    /// The query the options are currently filtered by (after the debounce).
    pub applied: String,
    /// The last text seen in the search field.
    pub seen: String,
    /// Seconds until `seen` is applied; `None` when nothing is waiting.
    pub wait: Option<f32>,
    /// After applying, move focus onto the first (or the selected) visible option.
    pub focus_item: bool,
    /// After applying, choose the first (or the selected) visible option.
    pub choose: bool,
}

/// Scroll the selected option into view once the opened menu has been laid out.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ComboboxReveal(pub u8);

/// Marker component for the header button that toggles the dropdown list.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxHeader(pub Entity);

/// Marker component for the text display inside the header.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxLabelText(pub Entity);

/// Marker component for the dropdown popup menu container.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxMenu(pub Entity);

/// Marker component for an individual selectable item in the combobox menu.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxMenuItem {
    pub combobox_entity: Entity,
    pub index: usize,
}

/// Builder for constructing Material Design 3 combobox widgets.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_combobox(mut commands: Commands) {
///     ComboboxBuilder::new("Route")
///         .with_options(vec!["Line 100".into(), "Line 200".into()])
///         .with_selected(0)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ComboboxBuilder {
    pub label: String,
    pub placeholder: String,
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub disabled: bool,
    /// A menu with more options than this gets a search field.
    pub search_threshold: usize,
}

impl ComboboxBuilder {
    /// Creates a new combobox builder with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            placeholder: String::new(),
            options: Vec::new(),
            selected: None,
            disabled: false,
            search_threshold: DEFAULT_SEARCH_THRESHOLD,
        }
    }

    /// Menus with more than `n` options get a search field (default 8; `usize::MAX` turns the
    /// search off).
    pub fn with_search_threshold(mut self, n: usize) -> Self {
        self.search_threshold = n;
        self
    }

    /// Sets the placeholder text shown when no item is selected.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the selectable options list.
    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    /// Sets the initially selected option index.
    pub fn with_selected(mut self, index: usize) -> Self {
        if index < self.options.len() {
            self.selected = Some(index);
        }
        self
    }

    /// Sets whether the combobox is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn current_display_text(&self) -> String {
        if let Some(idx) = self.selected {
            self.options
                .get(idx)
                .cloned()
                .unwrap_or_else(|| self.placeholder.clone())
        } else {
            self.placeholder.clone()
        }
    }

    fn root_bundle(&self) -> impl Bundle {
        (
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(200.0),
                position_type: PositionType::Relative,
                ..default()
            }),
            M3Combobox {
                options: self.options.clone(),
                selected: self.selected,
                placeholder: self.placeholder.clone(),
                is_open: false,
                search_threshold: self.search_threshold,
            },
        )
    }

    /// Spawns the combobox into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let display_text = self.current_display_text();
        let options = self.options.clone();
        let disabled = self.disabled;
        let search = options.len() > self.search_threshold;

        let mut root_cmd = commands.spawn(self.root_bundle());
        if disabled {
            root_cmd.insert(Disabled);
        }
        if search {
            root_cmd.insert(ComboboxFilter::default());
        }
        let root = root_cmd.id();

        commands.entity(root).with_children(|parent| {
            populate_combobox_children(parent, root, &display_text, &options, disabled, search);
        });

        root
    }

    /// Spawns the combobox as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let display_text = self.current_display_text();
        let options = self.options.clone();
        let disabled = self.disabled;
        let search = options.len() > self.search_threshold;

        let mut root_cmd = parent.spawn(self.root_bundle());
        if disabled {
            root_cmd.insert(Disabled);
        }
        if search {
            root_cmd.insert(ComboboxFilter::default());
        }
        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            populate_combobox_children(child_parent, root, &display_text, &options, disabled, search);
        });

        root
    }
}

fn populate_combobox_children(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    display_text: &str,
    options: &[String],
    disabled: bool,
    search: bool,
) {
    // Header button
    let mut header = parent.spawn((
        (
            Button,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                min_height: Val::Px(48.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
        ),
        ComboboxHeader(root),
        BackgroundRole(ColorRole::Surface),
        BorderRole(ColorRole::Outline),
        RadiusToken(RadiusLevel::ExtraSmall),
    ));
    if disabled {
        header.insert(Disabled);
    }
    header.with_children(|header_parent| {
        header_parent.spawn((
            (
                Text::new(display_text),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor::default(),
            ),
            ComboboxLabelText(root),
            ForegroundRole(ColorRole::OnSurface),
            TypeStyleToken(TypeScale::BodyMedium),
        ));

        header_parent.spawn((
            (
                Text::new("▼"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor::default(),
            ),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::Label),
            crate::ui_tree::A11yHidden,
        ));
    });

    // Dropdown menu (initially hidden): [search field] + scrolling list [+ "no match" row].
    parent
        .spawn((
            (
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    position_type: PositionType::Absolute,
                    top: Val::Px(52.0),
                    left: Val::Px(0.0),
                    min_width: Val::Percent(100.0),
                    max_height: Val::Px(if search { 288.0 } else { 240.0 }),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                GlobalZIndex(50),
            ),
            ComboboxMenu(root),
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            BorderRole(ColorRole::OutlineVariant),
            RadiusToken(RadiusLevel::ExtraSmall),
            ElevationToken(ElevationLevel::Level2),
        ))
        .observe(menu_wheel_scroll)
        .with_children(|menu_parent| {
            if search {
                let field = TextFieldBuilder::outlined("")
                    .compact()
                    .full_width()
                    .spawn_child(menu_parent);
                let mut field = menu_parent.commands_mut().entity(field);
                field.insert((
                    ComboboxSearch(root),
                    crate::l10n::L10nKey(crate::l10n::keys::COMBOBOX_SEARCH_LABEL),
                ));
            }
            menu_parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        // Long lists (a map with 16 start points) scroll with the wheel; the
                        // focused item is kept in view by `scroll_into_view`.
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                    ComboboxList(root),
                ))
                .with_children(|list| {
                    for (i, opt) in options.iter().enumerate() {
                        list.spawn((
                            (
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                                    width: Val::Percent(100.0),
                                    flex_shrink: 0.0,
                                    display: Display::Flex,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            ComboboxMenuItem {
                                combobox_entity: root,
                                index: i,
                            },
                            BackgroundRole(ColorRole::SurfaceContainerHigh),
                            RadiusToken(RadiusLevel::ExtraSmall),
                        ))
                        .with_children(|item_parent| {
                            item_parent.spawn((
                                (
                                    Text::new(opt.clone()),
                                    TextFont {
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor::default(),
                                ),
                                ForegroundRole(ColorRole::OnSurface),
                                TypeStyleToken(TypeScale::BodyMedium),
                            ));
                        });
                    }
                });
            if search {
                menu_parent.spawn((
                    (
                        Node {
                            display: Display::None,
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    crate::l10n::L10nKey(crate::l10n::keys::COMBOBOX_NO_MATCH),
                    ComboboxNoMatch(root),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));
            }
        });
}

/// Lower-case text without diacritics, for matching: `Süd` and `sud`, `Café` and `cafe`, `ß` and
/// `ss` compare equal.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{0300}'..='\u{036f}' => {}
            'ß' => out.push_str("ss"),
            'æ' | 'Æ' => out.push_str("ae"),
            'œ' | 'Œ' => out.push_str("oe"),
            _ => {
                for l in c.to_lowercase() {
                    out.push(base_letter(l));
                }
            }
        }
    }
    out
}

/// The unaccented base of a precomposed Latin letter (other characters stay).
fn base_letter(c: char) -> char {
    const TABLE: &[(&str, char)] = &[
        ("àáâãäåāăą", 'a'),
        ("çćĉċč", 'c'),
        ("ďđ", 'd'),
        ("èéêëēĕėęě", 'e'),
        ("ĝğġģ", 'g'),
        ("ĥħ", 'h'),
        ("ìíîïĩīĭįı", 'i'),
        ("ĵ", 'j'),
        ("ķ", 'k'),
        ("ĺļľŀł", 'l'),
        ("ñńņňŉ", 'n'),
        ("òóôõöøōŏő", 'o'),
        ("ŕŗř", 'r'),
        ("śŝşš", 's'),
        ("ţťŧ", 't'),
        ("ùúûüũūŭůűų", 'u'),
        ("ŵ", 'w'),
        ("ýÿŷ", 'y'),
        ("źżž", 'z'),
    ];
    TABLE
        .iter()
        .find(|(set, _)| set.contains(c))
        .map_or(c, |(_, base)| *base)
}

/// Whether `option` matches `query`: every whitespace-separated word of the query occurs in it
/// (case- and diacritics-insensitive). An empty query matches everything.
pub fn matches_query(option: &str, query: &str) -> bool {
    let hay = fold(option);
    fold(query)
        .split_whitespace()
        .all(|word| hay.contains(word))
}

/// Mouse-wheel scrolling of the open menu (the event does not travel on to the page behind it).
fn menu_wheel_scroll(
    mut ev: On<PointerScroll>,
    menus: Query<&ComboboxMenu>,
    mut lists: Query<(&ComboboxList, &mut ScrollPosition)>,
) {
    let Ok(menu) = menus.get(ev.entity) else {
        return;
    };
    let amount = match ev.unit {
        MouseScrollUnit::Line => ev.y * 24.0,
        MouseScrollUnit::Pixel => ev.y,
    };
    for (list, mut pos) in &mut lists {
        if list.0 == menu.0 {
            pos.y = (pos.y - amount).max(0.0);
        }
    }
    ev.propagate(false);
}

/// System handling clicks on the header button to toggle dropdown visibility.
pub fn combobox_header_system(
    mut activated: MessageReader<Activated>,
    header_query: Query<&ComboboxHeader>,
    mut combo_query: Query<&mut M3Combobox, Without<Disabled>>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(header) = header_query.get(*entity) {
            if let Ok(mut combo) = combo_query.get_mut(header.0) {
                combo.is_open = !combo.is_open;
            }
        }
    }
}

/// System handling clicks on dropdown menu items to select options.
pub fn combobox_item_system(
    mut activated: MessageReader<Activated>,
    item_query: Query<&ComboboxMenuItem>,
    headers: Query<(Entity, &ComboboxHeader)>,
    mut combo_query: Query<&mut M3Combobox, Without<Disabled>>,
    mut events: MessageWriter<ComboboxSelected>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(item) = item_query.get(*entity) {
            if let Ok(mut combo) = combo_query.get_mut(item.combobox_entity) {
                combo.selected = Some(item.index);
                combo.is_open = false;
                let val = combo.options.get(item.index).cloned().unwrap_or_default();
                events.write(ComboboxSelected {
                    entity: item.combobox_entity,
                    selected_index: item.index,
                    selected_value: val,
                });
                if let Some((header, _)) = headers.iter().find(|(_, h)| h.0 == item.combobox_entity)
                {
                    focus.write(FocusEvent::Set(header));
                }
            }
        }
    }
}

/// Up/Down on a focused, closed combobox header opens the list (the arrows are the header's own,
/// see `ArrowOwners` in the focus module).
pub fn combobox_arrow_system(
    mut actions: MessageReader<UiAction>,
    focused: Query<&ComboboxHeader, With<Focused>>,
    mut combo_query: Query<&mut M3Combobox, Without<Disabled>>,
) {
    for action in actions.read() {
        if matches!(action, UiAction::Navigate(Direction::Down | Direction::Up)) {
            if let Some(header) = focused.iter().next() {
                if let Ok(mut combo) = combo_query.get_mut(header.0) {
                    combo.is_open = true;
                }
            }
        }
    }
}

/// System updating visibility of dropdown menu and updating display text. A list that opens
/// while keyboard focus is visible moves focus onto the selected (else first) item; a searchable
/// list always focuses its search field instead and scrolls the selected item into view.
#[allow(clippy::too_many_arguments)]
pub fn combobox_sync_system(
    visible: Option<Res<InputFocusVisible>>,
    items: Query<(Entity, &ComboboxMenuItem)>,
    mut focus: MessageWriter<FocusEvent>,
    loc: Option<Res<Localization>>,
    combo_query: Query<(Entity, Ref<M3Combobox>)>,
    mut menu_query: Query<(&ComboboxMenu, &mut Node)>,
    mut text_query: Query<(&ComboboxLabelText, &mut Text)>,
    mut searches: Query<(&ComboboxSearch, &FocusProxy, &mut M3TextField)>,
    mut commands: Commands,
) {
    let loc_changed = loc.as_ref().is_some_and(|l| l.is_changed());
    for (entity, combo) in &combo_query {
        if !combo.is_changed() && !loc_changed {
            continue;
        }
        if combo.is_changed() && combo.is_open {
            commands.entity(entity).try_insert(ComboboxReveal(0));
            let search = searches.iter().find(|(s, _, _)| s.0 == entity);
            if let Some((_, proxy, _)) = search {
                focus.write(FocusEvent::Set(proxy.0));
            } else if visible.as_ref().is_none_or(|v| v.0) {
                let wanted = combo.selected.unwrap_or(0);
                if let Some((item, _)) = items
                    .iter()
                    .find(|(_, i)| i.combobox_entity == entity && i.index == wanted)
                {
                    focus.write(FocusEvent::Set(item));
                }
            }
        }
        if combo.is_changed() && !combo.is_open {
            // The next opening starts with every option.
            for (s, _, mut field) in &mut searches {
                if s.0 == entity && !field.text.is_empty() {
                    field.text.clear();
                }
            }
        }
        for (menu, mut style) in &mut menu_query {
            if menu.0 == entity {
                style.display = if combo.is_open {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }

        let display_str = combo
            .selected
            .and_then(|idx| combo.options.get(idx))
            .cloned()
            .unwrap_or_else(|| {
                if combo.placeholder.is_empty() {
                    loc.as_ref()
                        .map(|l| l.t(crate::l10n::keys::COMBOBOX_PLACEHOLDER))
                        .unwrap_or_default()
                } else {
                    combo.placeholder.clone()
                }
            });

        for (lbl, mut text) in &mut text_query {
            if lbl.0 == entity {
                text.0 = display_str.clone();
            }
        }
    }
}

/// Gives the search field its localised accessible name and placeholder, and the search input
/// the vertical arrows (Down moves into the list, see [`combobox_search_keys_system`]).
pub fn combobox_search_text_system(
    loc: Option<Res<Localization>>,
    mut fields: Query<(&mut M3TextField, &FocusProxy, Ref<ComboboxSearch>)>,
    mut commands: Commands,
) {
    let loc_changed = loc.as_ref().is_some_and(|l| l.is_changed());
    for (mut field, proxy, search) in &mut fields {
        if search.is_added() {
            commands.entity(proxy.0).try_insert(OwnsVerticalArrows);
        }
        if !(search.is_added() || loc_changed) {
            continue;
        }
        if let Some(loc) = loc.as_ref() {
            field.label = loc.t(crate::l10n::keys::COMBOBOX_SEARCH_LABEL);
            field.placeholder = loc.t(crate::l10n::keys::COMBOBOX_SEARCH);
        }
    }
}

/// Down in the search field moves into the (filtered) list; Enter chooses the selected option if
/// it still matches, else the first match. Both wait for the filter to be applied first.
pub fn combobox_search_keys_system(
    mut actions: MessageReader<UiAction>,
    mut activated: MessageReader<Activated>,
    inputs: Query<&TextFieldInput>,
    focused: Query<&TextFieldInput, With<Focused>>,
    searches: Query<&ComboboxSearch>,
    mut filters: Query<&mut ComboboxFilter>,
) {
    let mut request = |frame: Entity, choose: bool| {
        if let Ok(search) = searches.get(frame) {
            if let Ok(mut f) = filters.get_mut(search.0) {
                f.wait = Some(0.0);
                if choose {
                    f.choose = true;
                } else {
                    f.focus_item = true;
                }
            }
        }
    };
    for action in actions.read() {
        if matches!(action, UiAction::Navigate(Direction::Down)) {
            if let Some(input) = focused.iter().next() {
                request(input.0, false);
            }
        }
    }
    for Activated(entity) in activated.read() {
        if let Ok(input) = inputs.get(*entity) {
            request(input.0, true);
        }
    }
}

/// Applies the search text to the options after the debounce, shows the "No match" row and acts
/// on a pending Down / Enter.
#[allow(clippy::too_many_arguments)]
pub fn combobox_filter_system(
    time: Option<Res<Time>>,
    mut roots: Query<(Entity, &M3Combobox, &mut ComboboxFilter)>,
    searches: Query<(&ComboboxSearch, &M3TextField)>,
    mut items: Query<(Entity, &ComboboxMenuItem, &mut Node), Without<ComboboxNoMatch>>,
    mut no_match: Query<(&ComboboxNoMatch, &mut Node), Without<ComboboxMenuItem>>,
    mut focus: MessageWriter<FocusEvent>,
    mut activated: MessageWriter<Activated>,
) {
    let dt = time.map_or(0.0, |t| t.delta_secs());
    for (root, combo, mut filter) in &mut roots {
        let Some((_, field)) = searches.iter().find(|(s, _)| s.0 == root) else {
            continue;
        };
        if field.text != filter.seen {
            filter.seen.clone_from(&field.text);
            filter.wait = Some(if field.text.is_empty() {
                0.0
            } else {
                FILTER_DEBOUNCE_SECS
            });
        }
        let Some(wait) = filter.wait else { continue };
        if wait > dt {
            filter.wait = Some(wait - dt);
            continue;
        }
        let query = filter.seen.clone();
        filter.wait = None;
        filter.applied.clone_from(&query);
        let mut visible = Vec::new();
        for (entity, item, mut node) in &mut items {
            if item.combobox_entity != root {
                continue;
            }
            let shown = combo
                .options
                .get(item.index)
                .is_some_and(|o| matches_query(o, &query));
            let display = if shown { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
            if shown {
                visible.push((item.index, entity));
            }
        }
        for (n, mut node) in &mut no_match {
            if n.0 == root {
                let display = if visible.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
        let target = visible
            .iter()
            .find(|(i, _)| Some(*i) == combo.selected)
            .or_else(|| visible.iter().min_by_key(|(i, _)| *i))
            .map(|(_, e)| *e);
        if let Some(target) = target {
            if filter.choose {
                activated.write(Activated(target));
            } else if filter.focus_item {
                focus.write(FocusEvent::Set(target));
            }
        }
        filter.choose = false;
        filter.focus_item = false;
    }
}

/// Scrolls the selected option into view a frame after the menu opened (when it has a layout).
pub fn combobox_reveal_system(
    mut commands: Commands,
    mut roots: Query<(Entity, &M3Combobox, &mut ComboboxReveal)>,
    items: Query<(Entity, &ComboboxMenuItem)>,
    parents: Query<&ChildOf>,
    geometry: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut lists: Query<(Entity, &ComboboxList, &mut ScrollPosition)>,
) {
    for (root, combo, mut reveal) in &mut roots {
        if !combo.is_open {
            commands.entity(root).try_remove::<ComboboxReveal>();
            continue;
        }
        if reveal.0 < 1 {
            reveal.0 += 1;
            continue;
        }
        commands.entity(root).try_remove::<ComboboxReveal>();
        let Some(selected) = combo.selected else {
            continue;
        };
        let Some((item, _)) = items
            .iter()
            .find(|(_, i)| i.combobox_entity == root && i.index == selected)
        else {
            continue;
        };
        let Some((list, _, mut position)) = lists.iter_mut().find(|(_, l, _)| l.0 == root) else {
            continue;
        };
        if parents.get(item).map(ChildOf::parent) != Ok(list) {
            continue;
        }
        let (Ok((ic, it)), Ok((lc, lt))) = (geometry.get(item), geometry.get(list)) else {
            continue;
        };
        let rect = |c: &ComputedNode, t: &UiGlobalTransform| {
            let k = c.inverse_scale_factor;
            let half = c.size() / 2.0;
            ((t.translation - half) * k, (t.translation + half) * k)
        };
        let (imin, imax) = rect(ic, it);
        let (lmin, lmax) = rect(lc, lt);
        if imax.y - imin.y <= 0.0 || lmax.y - lmin.y <= 0.0 {
            continue;
        }
        if imin.y < lmin.y || imax.y > lmax.y {
            // Centre it when there is room.
            let slack = ((lmax.y - lmin.y) - (imax.y - imin.y)).max(0.0) / 2.0;
            position.y = (position.y + (imin.y - lmin.y) - slack).max(0.0);
        }
    }
}

/// Plugin registering Combobox widget systems and events.
#[derive(Default)]
pub struct ComboboxPlugin;

impl Plugin for ComboboxPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Messages<ComboboxSelected>>()
        {
            app.add_message::<ComboboxSelected>();
        }
        app.add_systems(
            Update,
            (
                combobox_header_system,
                combobox_arrow_system,
                combobox_item_system,
                combobox_sync_system,
            ),
        );
        app.add_systems(Update, combobox_search_systems());
    }
}

/// The systems of the search field of long menus.
pub fn combobox_search_systems() -> impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, ()>
{
    (
        combobox_search_text_system,
        combobox_search_keys_system,
        combobox_filter_system,
        combobox_reveal_system,
    )
        .chain()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combobox_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            ComboboxBuilder::new("Route")
                .with_options(vec!["Line 100".into(), "Line 200".into()])
                .with_selected(1)
                .spawn(&mut commands)
        };

        app.update();

        let combo = app.world().get::<M3Combobox>(entity);
        assert_eq!(combo.map(|c| c.selected), Some(Some(1)));
        assert_eq!(combo.map(|c| c.is_open), Some(false));
    }

    #[test]
    fn a_long_menu_scrolls_instead_of_clipping() {
        let mut app = App::new();
        let options: Vec<String> = (0..16).map(|i| format!("Start point {i}")).collect();
        {
            let mut commands = app.world_mut().commands();
            ComboboxBuilder::new("Start")
                .with_options(options)
                .spawn(&mut commands);
        }
        app.update();

        let mut lists = app
            .world_mut()
            .query_filtered::<(&Node, &ScrollPosition), With<ComboboxList>>();
        let (node, _) = lists.single(app.world()).expect("one list");
        assert_eq!(
            node.overflow.y,
            OverflowAxis::Scroll,
            "a list taller than the menu must scroll (the 16 start points of Berlin-Spandau)"
        );
    }

    #[test]
    fn folding_ignores_case_and_diacritics() {
        assert_eq!(fold("Süd-Straße Café Œuvre"), "sud-strasse cafe oeuvre");
        assert!(matches_query("Berlin Südkreuz", "sud"));
        assert!(matches_query("Berlin Südkreuz", "SÜD kreuz"));
        assert!(matches_query("Kreuzberg", "kreuz berg"));
        assert!(!matches_query("Berlin Südkreuz", "nord"));
        assert!(matches_query("anything", "   "));
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;
    use crate::actions::{Direction, UiAction};
    use crate::components::text_field::TextFieldClear;
    use crate::testing::{act, focused, headless_app, wheel};
    use std::time::Duration;

    const STEP_MS: u64 = 50;

    fn app_with(options: usize, selected: Option<usize>) -> (App, Entity) {
        let mut app = headless_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(STEP_MS),
        ));
        let opts: Vec<String> = (0..options).map(|i| format!("Start point {i}")).collect();
        let combo = {
            let mut commands = app.world_mut().commands();
            let mut b = ComboboxBuilder::new("Start").with_options(opts);
            if let Some(i) = selected {
                b = b.with_selected(i);
            }
            b.spawn(&mut commands)
        };
        for _ in 0..3 {
            app.update();
        }
        (app, combo)
    }

    fn frames(app: &mut App, n: usize) {
        for _ in 0..n {
            app.update();
        }
    }

    fn open(app: &mut App, combo: Entity) {
        app.world_mut().get_mut::<M3Combobox>(combo).unwrap().is_open = true;
        frames(app, 4);
    }

    fn search_field(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<ComboboxSearch>>()
            .single(app.world())
            .expect("a search field")
    }

    fn type_into(app: &mut App, text: &str) {
        let field = search_field(app);
        app.world_mut().get_mut::<M3TextField>(field).unwrap().text = text.to_string();
        frames(app, 2);
    }

    /// Indices of the options that are shown.
    fn shown(app: &mut App) -> Vec<usize> {
        let mut q = app.world_mut().query::<(&ComboboxMenuItem, &Node)>();
        let mut v: Vec<usize> = q
            .iter(app.world())
            .filter(|(_, n)| n.display != Display::None)
            .map(|(i, _)| i.index)
            .collect();
        v.sort_unstable();
        v
    }

    fn no_match_shown(app: &mut App) -> bool {
        let mut q = app.world_mut().query::<(&ComboboxNoMatch, &Node)>();
        q.iter(app.world()).any(|(_, n)| n.display != Display::None)
    }

    fn state(app: &App, combo: Entity) -> (bool, Option<usize>) {
        let c = app.world().get::<M3Combobox>(combo).unwrap();
        (c.is_open, c.selected)
    }

    #[test]
    fn short_lists_stay_plain() {
        let (mut app, _) = app_with(8, None);
        let n = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxSearch>>()
            .iter(app.world())
            .count();
        assert_eq!(n, 0, "8 options is not more than the default threshold");
        let (mut app, _) = app_with(9, None);
        let n = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxSearch>>()
            .iter(app.world())
            .count();
        assert_eq!(n, 1);
    }

    #[test]
    fn the_threshold_is_configurable() {
        let mut app = headless_app();
        {
            let mut commands = app.world_mut().commands();
            ComboboxBuilder::new("x")
                .with_options((0..4).map(|i| i.to_string()).collect())
                .with_search_threshold(3)
                .spawn(&mut commands);
        }
        app.update();
        app.update();
        let n = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxSearch>>()
            .iter(app.world())
            .count();
        assert_eq!(n, 1);
    }

    #[test]
    fn opening_focuses_the_search_field_and_typing_filters_after_the_debounce() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        let field = search_field(&mut app);
        let input = app.world().get::<FocusProxy>(field).unwrap().0;
        assert_eq!(focused(&mut app), Some(input), "opening focuses the field");
        assert_eq!(shown(&mut app).len(), 16);

        app.world_mut().get_mut::<M3TextField>(field).unwrap().text = "point 1".into();
        frames(&mut app, 2);
        assert_eq!(shown(&mut app).len(), 16, "within the 150 ms debounce");
        frames(&mut app, 3);
        assert_eq!(
            shown(&mut app),
            vec![1, 10, 11, 12, 13, 14, 15],
            "after the debounce"
        );
        assert!(!no_match_shown(&mut app));
    }

    #[test]
    fn a_16_option_start_point_list_can_be_narrowed_to_one() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "START POINT 7");
        frames(&mut app, 4);
        assert_eq!(shown(&mut app), vec![7]);
    }

    #[test]
    fn no_match_shows_a_row_and_the_clear_button_restores_the_list() {
        let (mut app, combo) = app_with(16, Some(3));
        open(&mut app, combo);
        type_into(&mut app, "zzz");
        frames(&mut app, 4);
        assert!(shown(&mut app).is_empty());
        assert!(no_match_shown(&mut app));
        assert_eq!(state(&app, combo).1, Some(3), "the selection is kept");

        let clear = app
            .world_mut()
            .query_filtered::<(Entity, &TextFieldClear), ()>()
            .iter(app.world())
            .map(|(e, _)| e)
            .next()
            .expect("clear button");
        crate::testing::click(&mut app, clear);
        frames(&mut app, 3);
        assert_eq!(shown(&mut app).len(), 16);
        assert!(!no_match_shown(&mut app));
        assert_eq!(state(&app, combo), (true, Some(3)));
    }

    #[test]
    fn the_selection_survives_filtering_that_keeps_it_visible() {
        let (mut app, combo) = app_with(16, Some(12));
        open(&mut app, combo);
        type_into(&mut app, "1");
        frames(&mut app, 4);
        assert!(shown(&mut app).contains(&12));
        assert_eq!(state(&app, combo).1, Some(12));
    }

    #[test]
    fn arrow_down_enters_the_list_and_enter_picks() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "point 13");
        act(&mut app, UiAction::Navigate(Direction::Down));
        frames(&mut app, 2);
        let item = focused(&mut app).expect("an option has focus");
        assert_eq!(
            app.world().get::<ComboboxMenuItem>(item).map(|i| i.index),
            Some(13),
            "Down skips the debounce and lands on the first match"
        );
        act(&mut app, UiAction::Confirm);
        assert_eq!(state(&app, combo), (false, Some(13)));
    }

    #[test]
    fn enter_in_the_search_field_picks_the_first_match() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "point 1");
        act(&mut app, UiAction::Confirm);
        frames(&mut app, 3);
        assert_eq!(state(&app, combo), (false, Some(1)));
    }

    #[test]
    fn enter_with_no_match_does_nothing() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "zzz");
        act(&mut app, UiAction::Confirm);
        frames(&mut app, 3);
        assert_eq!(state(&app, combo), (true, None));
    }

    #[test]
    fn escape_clears_the_filter_first_then_closes() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "point 1");
        frames(&mut app, 4);
        assert_eq!(shown(&mut app).len(), 7);
        act(&mut app, UiAction::Back);
        frames(&mut app, 2);
        assert_eq!(state(&app, combo).0, true, "first Esc keeps the list open");
        assert_eq!(shown(&mut app).len(), 16, "and shows every option again");
        act(&mut app, UiAction::Back);
        assert_eq!(state(&app, combo).0, false, "second Esc closes it");
    }

    #[test]
    fn closing_resets_the_filter() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        type_into(&mut app, "point 1");
        frames(&mut app, 4);
        app.world_mut().get_mut::<M3Combobox>(combo).unwrap().is_open = false;
        frames(&mut app, 4);
        assert_eq!(shown(&mut app).len(), 16);
    }

    #[test]
    fn the_search_field_is_named_and_localised() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        let field = search_field(&mut app);
        let f = app.world().get::<M3TextField>(field).unwrap();
        assert_eq!(f.label, "Filter the options");
        assert_eq!(f.placeholder, "Search...");
        app.world_mut()
            .resource_mut::<Localization>()
            .set_locale("de");
        frames(&mut app, 2);
        let f = app.world().get::<M3TextField>(field).unwrap();
        assert_eq!(f.label, "Optionen filtern");
        assert_eq!(f.placeholder, "Suchen ...");
        app.world_mut()
            .resource_mut::<Localization>()
            .set_locale("en-XA");
        frames(&mut app, 2);
        let f = app.world().get::<M3TextField>(field).unwrap();
        assert!(f.label.contains('é') || f.label.contains('í') || f.label.contains('ö'));
    }

    #[test]
    fn the_search_field_has_a_stable_id_in_every_language() {
        let (mut app, combo) = app_with(16, None);
        app.world_mut()
            .entity_mut(combo)
            .insert(crate::ui_id::UiId::new("wizard.where.start-point"));
        open(&mut app, combo);
        frames(&mut app, 2);
        let field = search_field(&mut app);
        let id = |app: &App| app.world().get::<crate::ui_id::UiId>(field).map(|i| i.0.to_string());
        let en = id(&app);
        assert_eq!(
            en.as_deref(),
            Some("wizard.where.start-point/textbox:widget-combobox-search-label")
        );
        app.world_mut()
            .resource_mut::<Localization>()
            .set_locale("de");
        frames(&mut app, 3);
        assert_eq!(id(&app), en, "the id does not depend on the language");
    }

    #[test]
    fn the_wheel_still_scrolls_the_list_and_stays_in_the_menu() {
        let (mut app, combo) = app_with(16, None);
        open(&mut app, combo);
        let menu = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxMenu>>()
            .single(app.world())
            .unwrap();
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxList>>()
            .single(app.world())
            .unwrap();
        wheel(app.world_mut(), menu, 3.0);
        let y = app.world().get::<ScrollPosition>(list).unwrap().y;
        assert!(y > 0.0, "the wheel moved the list, got {y}");
        wheel(app.world_mut(), menu, -10.0);
        assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
    }
}

#[cfg(test)]
mod reveal_tests {
    use super::*;
    use crate::testing::layout_app;

    #[test]
    fn opening_scrolls_the_selected_option_into_view() {
        let mut app = layout_app();
        let combo = {
            let mut commands = app.world_mut().commands();
            ComboboxBuilder::new("Start")
                .with_options((0..16).map(|i| format!("Start point {i}")).collect())
                .with_selected(14)
                .spawn(&mut commands)
        };
        for _ in 0..4 {
            app.update();
        }
        app.world_mut().get_mut::<M3Combobox>(combo).unwrap().is_open = true;
        for _ in 0..8 {
            app.update();
        }
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<ComboboxList>>()
            .single(app.world())
            .unwrap();
        let y = app.world().get::<ScrollPosition>(list).unwrap().y;
        assert!(y > 0.0, "option 14 of 16 is below the fold, scroll is {y}");
    }
}
