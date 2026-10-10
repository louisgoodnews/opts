//! Modal dialog primitive: a full-screen backdrop with a centred content card.
//!
//! The backdrop blocks pointer interaction with everything beneath it (`FocusPolicy::Block`) and
//! clicking it does **nothing** by design (closing by backdrop click would discard user input).
//! Escape emits [`ModalEvent::CancelRequested`]; the owner of the modal decides what that means.
//! While a modal exists, Tab / Shift+Tab cycle only through the focusable widgets inside it.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::{
    interaction::{
        focus::{FocusEvent, Focusable},
        state::{Disabled, Focused},
    },
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
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
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
///         p.spawn(TextBundle::from_section("Hello", TextStyle::default()));
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
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    // Scrim at ~60 % opacity.
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    focus_policy: FocusPolicy::Block,
                    z_index: ZIndex::Global(self.z_index),
                    ..default()
                },
                // Interaction makes the backdrop swallow pointer events beneath it.
                Interaction::default(),
                ModalRoot,
                Name::new("ModalRoot"),
            ))
            .id();
        let content = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(self.width_percent),
                        height: Val::Percent(self.height_percent),
                        min_width: Val::Px(self.min_width),
                        max_width: Val::Px(self.max_width),
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    focus_policy: FocusPolicy::Block,
                    ..default()
                },
                Interaction::default(),
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
    mut events: EventWriter<ModalEvent>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Some(modal) = modals.iter().last() {
            events.send(ModalEvent::CancelRequested(modal));
        }
    }
}

fn is_inside(mut entity: Entity, ancestor: Entity, parents: &Query<&Parent>) -> bool {
    while let Ok(p) = parents.get(entity) {
        entity = p.get();
        if entity == ancestor {
            return true;
        }
    }
    false
}

/// Keeps Tab / Shift+Tab inside the modal: cycles the [`Focusable`]s below [`ModalContent`].
pub fn modal_focus_trap_system(
    keys: Res<ButtonInput<KeyCode>>,
    content: Query<Entity, With<ModalContent>>,
    focusables: Query<(Entity, &Focusable), Without<Disabled>>,
    focused: Query<Entity, With<Focused>>,
    parents: Query<&Parent>,
    mut events: EventWriter<FocusEvent>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    let Some(content) = content.iter().last() else {
        return;
    };
    let mut inside: Vec<(usize, Entity)> = focusables
        .iter()
        .filter(|(e, _)| is_inside(*e, content, &parents))
        .map(|(e, f)| (f.tab_index, e))
        .collect();
    if inside.is_empty() {
        return;
    }
    inside.sort();
    let backwards = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let current = focused
        .iter()
        .find_map(|f| inside.iter().position(|(_, e)| *e == f));
    let len = inside.len();
    let next = match (current, backwards) {
        (Some(i), false) => (i + 1) % len,
        (Some(i), true) => (i + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    };
    events.send(FocusEvent::Set(inside[next].1));
}

/// Plugin registering the modal systems and [`ModalEvent`].
#[derive(Default)]
pub struct ModalPlugin;

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ModalEvent>()
            .add_event::<FocusEvent>()
            .add_systems(Update, (modal_escape_system, modal_focus_trap_system));
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
            app.world().get::<Parent>(content).map(Parent::get),
            Some(root)
        );
    }

    #[test]
    fn escape_requests_cancel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_event::<ModalEvent>()
            .add_systems(Update, modal_escape_system);
        let root = app.world_mut().spawn(ModalRoot).id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let events = app.world().resource::<Events<ModalEvent>>();
        let got: Vec<_> = events.get_reader().read(events).copied().collect();
        assert_eq!(got, vec![ModalEvent::CancelRequested(root)]);
    }

    #[test]
    fn tab_stays_inside_the_modal() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_event::<FocusEvent>()
            .add_systems(Update, modal_focus_trap_system);
        let content = app.world_mut().spawn(ModalContent).id();
        let a = app
            .world_mut()
            .spawn(Focusable { tab_index: 0 })
            .set_parent(content)
            .id();
        let b = app
            .world_mut()
            .spawn(Focusable { tab_index: 1 })
            .set_parent(content)
            .id();
        let _outside = app.world_mut().spawn(Focusable { tab_index: 0 }).id();
        app.world_mut().entity_mut(b).insert(Focused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        let events = app.world().resource::<Events<FocusEvent>>();
        let got: Vec<_> = events.get_reader().read(events).copied().collect();
        assert_eq!(got, vec![FocusEvent::Set(a)]);
    }
}
