//! Material Design 3 TextField text input widget.

use bevy::prelude::*;

use crate::{
    interaction::{events::TextChanged, focus::Focusable, state::Disabled},
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// Variants for Material Design 3 text fields.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFieldVariant {
    #[default]
    Outlined,
    Filled,
}

/// Marker component for an M3 TextField entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3TextField {
    pub variant: TextFieldVariant,
    pub text: String,
    pub placeholder: String,
    pub is_focused: bool,
}

/// Builder for constructing Material Design 3 text fields.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_input(mut commands: Commands) {
///     TextFieldBuilder::outlined("Route Code")
///         .with_placeholder("e.g. 100")
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TextFieldBuilder {
    pub label: String,
    pub placeholder: String,
    pub initial_text: String,
    pub variant: TextFieldVariant,
}

impl TextFieldBuilder {
    /// Creates an outlined text field builder with a label.
    pub fn outlined(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            placeholder: String::new(),
            initial_text: String::new(),
            variant: TextFieldVariant::Outlined,
        }
    }

    /// Creates a filled text field builder with a label.
    pub fn filled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            placeholder: String::new(),
            initial_text: String::new(),
            variant: TextFieldVariant::Filled,
        }
    }

    /// Sets placeholder helper text displayed when input is empty.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the initial text content.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.initial_text = text.into();
        self
    }

    fn style(&self) -> Style {
        Style {
            min_height: Val::Px(48.0),
            min_width: Val::Px(200.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            border: if self.variant == TextFieldVariant::Outlined {
                UiRect::all(Val::Px(1.0))
            } else {
                UiRect {
                    bottom: Val::Px(2.0),
                    ..default()
                }
            },
            ..default()
        }
    }

    /// Spawns the text field entity hierarchy using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, border_role) = match self.variant {
            TextFieldVariant::Outlined => (ColorRole::Surface, ColorRole::Outline),
            TextFieldVariant::Filled => (ColorRole::SurfaceContainerHighest, ColorRole::Primary),
        };

        let style = self.style();
        let placeholder = self.placeholder;
        let initial_text = self.initial_text;
        let display_text = if initial_text.is_empty() {
            placeholder.clone()
        } else {
            initial_text.clone()
        };
        let is_placeholder = initial_text.is_empty() && !placeholder.is_empty();

        let entity_commands = commands.spawn((
            ButtonBundle { style, ..default() },
            M3TextField {
                variant: self.variant,
                text: initial_text,
                placeholder,
                is_focused: false,
            },
            BackgroundRole(bg_role),
            BorderRole(border_role),
            RadiusToken(RadiusLevel::ExtraSmall),
            Focusable { tab_index: 0 },
        ));

        let root = entity_commands.id();

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    display_text,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(if is_placeholder {
                    ColorRole::OnSurfaceVariant
                } else {
                    ColorRole::OnSurface
                }),
            ));
        });

        root
    }

    /// Spawns the text field as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        let (bg_role, border_role) = match self.variant {
            TextFieldVariant::Outlined => (ColorRole::Surface, ColorRole::Outline),
            TextFieldVariant::Filled => (ColorRole::SurfaceContainerHighest, ColorRole::Primary),
        };

        let style = self.style();
        let placeholder = self.placeholder;
        let initial_text = self.initial_text;
        let display_text = if initial_text.is_empty() {
            placeholder.clone()
        } else {
            initial_text.clone()
        };
        let is_placeholder = initial_text.is_empty() && !placeholder.is_empty();

        let mut entity_commands = parent.spawn((
            ButtonBundle { style, ..default() },
            M3TextField {
                variant: self.variant,
                text: initial_text,
                placeholder,
                is_focused: false,
            },
            BackgroundRole(bg_role),
            BorderRole(border_role),
            RadiusToken(RadiusLevel::ExtraSmall),
            Focusable { tab_index: 0 },
        ));

        let root = entity_commands.id();

        entity_commands.with_children(|child_parent| {
            child_parent.spawn((
                TextBundle::from_section(
                    display_text,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(if is_placeholder {
                    ColorRole::OnSurfaceVariant
                } else {
                    ColorRole::OnSurface
                }),
            ));
        });

        root
    }
}

/// Query item alias for text field interaction query.
pub type TextFieldInteractionItem<'a> = (Entity, &'a Interaction, &'a mut M3TextField);

/// Query filter alias for active text field interactions.
pub type TextFieldInteractionFilter = (Without<Disabled>, Changed<Interaction>);

/// System managing focus and click activation for [`M3TextField`] entities.
pub fn text_field_interaction_system(
    mut interaction_query: Query<TextFieldInteractionItem, TextFieldInteractionFilter>,
) {
    for (_entity, interaction, mut field) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            field.is_focused = true;
        }
    }
}

/// System detecting text modifications on [`M3TextField`] and dispatching [`TextChanged`].
pub fn text_field_change_system(
    query: Query<(Entity, &M3TextField), Changed<M3TextField>>,
    mut events: EventWriter<TextChanged>,
) {
    for (entity, field) in &query {
        events.send(TextChanged {
            entity,
            text: field.text.clone(),
        });
    }
}

/// Plugin registering TextField widget systems.
#[derive(Default)]
pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Events<TextChanged>>() {
            app.add_event::<TextChanged>();
        }
        app.add_systems(
            Update,
            (text_field_interaction_system, text_field_change_system),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_field_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            TextFieldBuilder::outlined("Route")
                .with_placeholder("Enter route")
                .with_text("100")
                .spawn(&mut commands)
        };

        app.update();

        let field = app.world().get::<M3TextField>(entity);
        assert_eq!(field.map(|f| f.text.as_str()), Some("100"));
        assert_eq!(field.map(|f| f.placeholder.as_str()), Some("Enter route"));
    }

    #[test]
    fn test_text_field_change_event() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<TextChanged>()
            .add_systems(Update, text_field_change_system);

        let entity = {
            let mut commands = app.world_mut().commands();
            TextFieldBuilder::outlined("Route").spawn(&mut commands)
        };

        app.update();

        // Mutate text
        if let Some(mut field) = app.world_mut().get_mut::<M3TextField>(entity) {
            field.text = "M29".into();
        }

        app.update();

        let events = app.world().resource::<Events<TextChanged>>();
        let mut reader = events.get_reader();
        let received = reader.read(events).filter(|e| e.entity == entity).last();
        assert_eq!(received.map(|e| e.text.as_str()), Some("M29"));
    }
}
