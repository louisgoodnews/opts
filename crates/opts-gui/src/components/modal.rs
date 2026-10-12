//! Modal dialog primitive: a full-screen backdrop with a centred content card.
//!
//! The backdrop blocks pointer interaction with everything beneath it (`FocusPolicy::Block`) and
//! clicking it does **nothing** by design (closing by backdrop click would discard user input).
//! Escape emits [`ModalEvent::CancelRequested`]; the owner of the modal decides what that means.
//! While a modal exists, focus navigation considers only the focusable widgets inside it
//! (`FocusGraph` scope) and focus returns to the previous control when it closes.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::{
    interaction::focus::FocusEvent,
    theme::{color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel},
    tokens::components::{BackgroundRole, ElevationToken, RadiusToken},
};

/// Marker on the full-screen backdrop node.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ModalRoot;

/// Marker on the content card inside a [`ModalRoot`].
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ModalContent;

/// Events emitted by modals.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalEvent {
    /// The user pressed Escape while the modal was open.
    CancelRequested(Entity),
}

/// Builder for a modal; children are added to the returned content entity.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn open(mut commands: Commands) {
///     let (root, content) = ModalBuilder::new().spawn(&mut commands);
///     commands.entity(content).with_children(|p| {
///         p.spawn((Text::new("Hello")));
///     });
///     let _ = root;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ModalBuilder {
    width_percent: f32,
    height_percent: f32,
    min_width: f32,
    max_width: f32,
    z_index: i32,
}

impl Default for ModalBuilder {
    fn default() -> Self {
        Self {
            width_percent: 70.0,
            height_percent: 90.0,
            min_width: 600.0,
            max_width: 1200.0,
            z_index: 1000,
        }
    }
}

impl ModalBuilder {
    /// Creates a modal builder with the default card size (70 % × 90 %, 600–1200 px wide).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the card width and height as percentages of the viewport.
    #[must_use]
    pub fn with_size_percent(mut self, width: f32, height: f32) -> Self {
        self.width_percent = width;
        self.height_percent = height;
        self
    }

    /// Sets the card's minimum and maximum width in logical pixels.
    #[must_use]
    pub fn with_width_range(mut self, min: f32, max: f32) -> Self {
        self.min_width = min;
        self.max_width = max;
        self
    }

    /// Spawns the modal and returns `(backdrop, content_card)`.
    pub fn spawn(self, commands: &mut Commands) -> (Entity, Entity) {
        let root = commands
            .spawn((
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    FocusPolicy::Block,
                    GlobalZIndex(self.z_index),
                ),
                ModalRoot,
                Name::new("ModalRoot"),
            ))
            .id();
        let content = commands
            .spawn((
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(self.width_percent),
                        height: Val::Percent(self.height_percent),
                        min_width: Val::Px(self.min_width),
                        max_width: Val::Px(self.max_width),
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    FocusPolicy::Block,
                ),
                ModalContent,
                BackgroundRole(ColorRole::SurfaceContainerHigh),
                RadiusToken(RadiusLevel::Large),
                ElevationToken(ElevationLevel::Level3),
                Name::new("ModalContent"),
            ))
            .id();
        commands.entity(root).add_child(content);
        (root, content)
    }
}

/// Emits [`ModalEvent::CancelRequested`] for the topmost modal when Escape is pressed.
pub fn modal_escape_system(
    keys: Res<ButtonInput<KeyCode>>,
    modals: Query<Entity, With<ModalRoot>>,
    mut events: MessageWriter<ModalEvent>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Some(modal) = modals.iter().last() {
            events.write(ModalEvent::CancelRequested(modal));
        }
    }
}

/// Plugin registering the modal systems and [`ModalEvent`].
#[derive(Default)]
pub struct ModalPlugin;

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ModalEvent>()
            .add_message::<FocusEvent>()
            .add_systems(Update, modal_escape_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_spawns_card_under_backdrop() {
        let mut app = App::new();
        let (root, content) = {
            let mut commands = app.world_mut().commands();
            ModalBuilder::new().spawn(&mut commands)
        };
        app.update();
        assert!(app.world().get::<ModalRoot>(root).is_some());
        assert!(app.world().get::<ModalContent>(content).is_some());
        assert_eq!(
            app.world().get::<ChildOf>(content).map(ChildOf::parent),
            Some(root)
        );
    }

    #[test]
    fn escape_requests_cancel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<ModalEvent>()
            .add_systems(Update, modal_escape_system);
        let root = app.world_mut().spawn(ModalRoot).id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let events = app.world().resource::<Messages<ModalEvent>>();
        let got: Vec<_> = events.get_cursor().read(events).copied().collect();
        assert_eq!(got, vec![ModalEvent::CancelRequested(root)]);
    }
}
