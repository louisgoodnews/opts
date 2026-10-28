//! Material Design 3 Button widget with variant styling and click actions.

use bevy::prelude::*;
use std::sync::Arc;

use crate::{
    interaction::{
        activation::Activated,
        events::ButtonClicked,
        focus::Focusable,
        layer::StateLayered,
        state::{Disabled, Hovered},
    },
    l10n::L10nKey,
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

/// Marker for an interactive control: every widget root carries it. It requires the pointer
/// state Bevy's picking keeps current ([`Hovered`]) and `Node`; the state layer, focus ring and
/// disabled treatment of [`crate::interaction::layer`] attach to it, and pointer release inside
/// the control sends [`Activated`].
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(Node, bevy::ui::FocusPolicy::Block, Hovered, Focusable, StateLayered)]
pub struct Button;

/// Button variants supported by Material Design 3.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Filled,
    Elevated,
    FilledTonal,
    Outlined,
    Text,
    Icon,
}

/// Type alias for button click action callback closures.
pub type ButtonClickHandler = Arc<dyn Fn(&mut Commands, Entity) + Send + Sync>;

/// Optional user callback executed when an M3 button is clicked.
#[derive(Component, Clone)]
pub struct OnClickCallback(pub ButtonClickHandler);

/// Marker component for a Material Design 3 button root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Button {
    pub variant: ButtonVariant,
}

/// Builder for constructing Material Design 3 buttons with fluent configuration.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_my_button(mut commands: Commands) {
///     ButtonBuilder::filled("Click Me")
///         .on_click(|_commands, _entity| {
///             // Action
///         })
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Clone)]
pub struct ButtonBuilder {
    pub label: String,
    pub variant: ButtonVariant,
    pub icon: Option<Handle<Image>>,
    pub is_disabled: bool,
    pub on_click: Option<ButtonClickHandler>,
    /// Message key of the label: the text follows the active locale.
    pub label_key: Option<&'static str>,
}

impl ButtonBuilder {
    /// Creates a new filled button builder.
    pub fn filled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Filled,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new elevated button builder.
    pub fn elevated(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Elevated,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new filled tonal button builder.
    pub fn tonal(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::FilledTonal,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new outlined button builder.
    pub fn outlined(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Outlined,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new text button builder.
    pub fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Text,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a circular icon button builder.
    pub fn icon(icon: Handle<Image>) -> Self {
        Self {
            label: String::new(),
            variant: ButtonVariant::Icon,
            icon: Some(icon),
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Makes the label a message key (`label` stays the fallback text until it is resolved).
    pub fn with_key(mut self, key: &'static str) -> Self {
        self.label_key = Some(key);
        self
    }

    /// Attaches an optional leading icon asset.
    pub fn with_icon(mut self, icon: Handle<Image>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Marks the button as disabled (non-interactive).
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.is_disabled = disabled;
        self
    }

    /// Attaches an `on_click` action callback that runs whenever the button is activated.
    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Commands, Entity) + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(callback));
        self
    }

    fn roles(&self) -> (ColorRole, ColorRole, ElevationLevel) {
        match self.variant {
            ButtonVariant::Filled => (
                ColorRole::Primary,
                ColorRole::OnPrimary,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Elevated => (
                ColorRole::SurfaceContainerLow,
                ColorRole::Primary,
                ElevationLevel::Level1,
            ),
            ButtonVariant::FilledTonal => (
                ColorRole::SecondaryContainer,
                ColorRole::OnSecondaryContainer,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Outlined => (
                ColorRole::Surface,
                ColorRole::Primary,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Text => (
                ColorRole::Surface,
                ColorRole::Primary,
                ElevationLevel::Level0,
            ),
            ButtonVariant::Icon => (
                ColorRole::SurfaceContainerHighest,
                ColorRole::OnSurfaceVariant,
                ElevationLevel::Level0,
            ),
        }
    }

    fn style(&self) -> Node {
        let is_icon_only = self.variant == ButtonVariant::Icon;
        let padding = if is_icon_only {
            UiRect::all(Val::Px(8.0))
        } else {
            UiRect::axes(Val::Px(24.0), Val::Px(10.0))
        };
        Node {
            min_height: Val::Px(40.0),
            min_width: if is_icon_only {
                Val::Px(40.0)
            } else {
                Val::Auto
            },
            padding,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: if self.variant == ButtonVariant::Outlined {
                UiRect::all(Val::Px(1.0))
            } else {
                UiRect::DEFAULT
            },
            ..default()
        }
    }

    /// Spawns the button entity hierarchy using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, fg_role, elevation) = self.roles();
        let mut entity_commands = commands.spawn((
            (Button, self.style()),
            M3Button {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Full),
            ElevationToken(elevation),
            Focusable { tab_index: 0 },
        ));

        if self.variant == ButtonVariant::Outlined {
            entity_commands.insert(BorderRole(ColorRole::Outline));
        }

        if self.is_disabled {
            entity_commands.insert(Disabled);
        }

        if let Some(cb) = self.on_click {
            entity_commands.insert(OnClickCallback(cb));
        }

        let button_entity = entity_commands.id();

        if let Some(icon_handle) = self.icon {
            commands.entity(button_entity).with_children(|parent| {
                parent.spawn((
                    ImageNode::new(icon_handle),
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                ));
            });
        }

        if !self.label.is_empty() {
            let label_text = self.label;
            let key = self.label_key;
            commands.entity(button_entity).with_children(|parent| {
                let mut label = parent.spawn((
                    (
                        Text::new(label_text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(fg_role),
                ));
                if let Some(key) = key {
                    label.insert(L10nKey(key));
                }
            });
        }

        button_entity
    }

    /// Spawns the button entity hierarchy as a child inside a [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let (bg_role, fg_role, elevation) = self.roles();
        let style = self.style();
        let icon_handle = self.icon;
        let label_text = self.label;
        let is_outlined = self.variant == ButtonVariant::Outlined;
        let is_disabled = self.is_disabled;
        let on_click = self.on_click;
        let label_key = self.label_key;

        let mut entity_commands = parent.spawn((
            (Button, style),
            M3Button {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Full),
            ElevationToken(elevation),
            Focusable { tab_index: 0 },
        ));

        if is_outlined {
            entity_commands.insert(BorderRole(ColorRole::Outline));
        }

        if is_disabled {
            entity_commands.insert(Disabled);
        }

        if let Some(cb) = on_click {
            entity_commands.insert(OnClickCallback(cb));
        }

        entity_commands.with_children(|child_parent| {
            if let Some(icon) = icon_handle {
                child_parent.spawn((
                    ImageNode::new(icon),
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                ));
            }

            if !label_text.is_empty() {
                let mut label = child_parent.spawn((
                    (
                        Text::new(label_text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(fg_role),
                ));
                if let Some(key) = label_key {
                    label.insert(L10nKey(key));
                }
            }
        });

        entity_commands.id()
    }
}

/// System turning [`Activated`] on every [`Button`] (M3 or hand-built) into [`ButtonClicked`]
/// events and callbacks.
pub fn button_click_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    buttons: Query<Option<&OnClickCallback>, (With<Button>, Without<Disabled>)>,
    mut click_events: MessageWriter<ButtonClicked>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(callback) = buttons.get(*entity) {
            click_events.write(ButtonClicked(*entity));
            if let Some(cb) = callback {
                (cb.0)(&mut commands, *entity);
            }
        }
    }
}

/// Plugin registering Button widget systems.
#[derive(Default)]
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ButtonClicked>()
            .add_message::<Activated>();
        app.add_systems(Update, button_click_system);
    }
}

#[cfg(test)]
mod tests;
