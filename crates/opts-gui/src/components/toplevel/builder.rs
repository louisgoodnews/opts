//! Spawning of a toplevel window: the root node with its scrim, the header and the body.

use bevy::prelude::*;

use crate::theme::color::ColorRole;
use crate::tokens::components::BackgroundRole;

use super::populate::populate_toplevel_children;
use super::M3Toplevel;

/// Builder for constructing Material Design 3 toplevel floating dialog windows.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_dialog(mut commands: Commands) {
///     ToplevelBuilder::new("Vehicle Dispatcher")
///         .with_size(480.0, 320.0)
///         .modal(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ToplevelBuilder {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub is_modal: bool,
}

impl ToplevelBuilder {
    /// Creates a new toplevel window builder with the specified title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 480.0,
            height: 360.0,
            is_modal: true,
        }
    }

    /// Sets the window dimensions in dp/pixels.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets whether the toplevel window displays a modal backdrop scrim.
    pub fn modal(mut self, is_modal: bool) -> Self {
        self.is_modal = is_modal;
        self
    }

    fn root_bundle(&self) -> impl Bundle {
        (
            (
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                GlobalZIndex(90),
                BackgroundColor(Color::NONE),
            ),
            M3Toplevel {
                title: self.title.clone(),
                is_modal: self.is_modal,
            },
        )
    }

    /// Spawns the toplevel window into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let is_modal = self.is_modal;
        let title = self.title.clone();
        let width = self.width;
        let height = self.height;

        let mut root_cmd = commands.spawn(self.root_bundle());
        if is_modal {
            root_cmd.insert(BackgroundRole(ColorRole::Scrim));
        }
        let root = root_cmd.id();

        commands.entity(root).with_children(|scrim_parent| {
            populate_toplevel_children(scrim_parent, root, &title, width, height);
        });

        root
    }

    /// Spawns the toplevel window as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let is_modal = self.is_modal;
        let title = self.title.clone();
        let width = self.width;
        let height = self.height;

        let mut root_cmd = parent.spawn(self.root_bundle());
        if is_modal {
            root_cmd.insert(BackgroundRole(ColorRole::Scrim));
        }
        let root = root_cmd.id();

        root_cmd.with_children(|scrim_parent| {
            populate_toplevel_children(scrim_parent, root, &title, width, height);
        });

        root
    }
}
