//! The combobox builder and the population of its children.

use super::menu::spawn_dropdown_menu;
use super::parts::spawn_header;
use super::search::{ComboboxFilter, DEFAULT_SEARCH_THRESHOLD};
use super::*;

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
            populate_combobox_children(
                child_parent,
                root,
                &display_text,
                &options,
                disabled,
                search,
            );
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
    spawn_header(parent, root, display_text, disabled);
    spawn_dropdown_menu(parent, root, options, search);
}
