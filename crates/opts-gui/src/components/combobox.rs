//! Material Design 3 Combobox / Dropdown selection widget.

use crate::components::Button;
use bevy::prelude::*;

use bevy::input_focus::InputFocusVisible;

use crate::{
    actions::{Direction, UiAction},
    interaction::{
        activation::Activated,
        events::ComboboxSelected,
        focus::FocusEvent,
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
}

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
}

impl ComboboxBuilder {
    /// Creates a new combobox builder with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            placeholder: "Select option...".into(),
            options: Vec::new(),
            selected: None,
            disabled: false,
        }
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
            },
        )
    }

    /// Spawns the combobox into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let display_text = self.current_display_text();
        let options = self.options.clone();
        let disabled = self.disabled;

        let mut root_cmd = commands.spawn(self.root_bundle());
        if disabled {
            root_cmd.insert(Disabled);
        }
        let root = root_cmd.id();

        commands.entity(root).with_children(|parent| {
            populate_combobox_children(parent, root, &display_text, &options, disabled);
        });

        root
    }

    /// Spawns the combobox as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let display_text = self.current_display_text();
        let options = self.options.clone();
        let disabled = self.disabled;

        let mut root_cmd = parent.spawn(self.root_bundle());
        if disabled {
            root_cmd.insert(Disabled);
        }
        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            populate_combobox_children(child_parent, root, &display_text, &options, disabled);
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
                TextColor(Color::WHITE),
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
                TextColor(Color::WHITE),
            ),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::LabelMedium),
            crate::ui_tree::A11yHidden,
        ));
    });

    // Dropdown menu (initially hidden)
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
                    max_height: Val::Px(240.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    overflow: Overflow::clip_y(),
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
        .with_children(|menu_parent| {
            for (i, opt) in options.iter().enumerate() {
                menu_parent
                    .spawn((
                        (
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                                width: Val::Percent(100.0),
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
                                TextColor(Color::WHITE),
                            ),
                            ForegroundRole(ColorRole::OnSurface),
                            TypeStyleToken(TypeScale::BodyMedium),
                        ));
                    });
            }
        });
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
/// while keyboard focus is visible moves focus onto the selected (else first) item.
pub fn combobox_sync_system(
    visible: Option<Res<InputFocusVisible>>,
    items: Query<(Entity, &ComboboxMenuItem)>,
    mut focus: MessageWriter<FocusEvent>,
    combo_query: Query<(Entity, &M3Combobox), Changed<M3Combobox>>,
    mut menu_query: Query<(&ComboboxMenu, &mut Node)>,
    mut text_query: Query<(&ComboboxLabelText, &mut Text)>,
) {
    for (entity, combo) in &combo_query {
        if combo.is_open && visible.as_ref().is_none_or(|v| v.0) {
            let wanted = combo.selected.unwrap_or(0);
            if let Some((item, _)) = items
                .iter()
                .find(|(_, i)| i.combobox_entity == entity && i.index == wanted)
            {
                focus.write(FocusEvent::Set(item));
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
            .unwrap_or_else(|| combo.placeholder.clone());

        for (lbl, mut text) in &mut text_query {
            if lbl.0 == entity {
                text.0 = display_str.clone();
            }
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
    }
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
}
