//! Material Design 3 Toplevel / Floating modal window dialog widget.

use bevy::prelude::*;

use crate::{
    interaction::events::ToplevelClosed,
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

/// Marker component for an M3 Toplevel floating window root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Toplevel {
    pub title: String,
    pub is_modal: bool,
}

/// Marker component for the window close button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelCloseButton(pub Entity);

/// Marker component for the inner content body of a toplevel window.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelBody(pub Entity);

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

fn populate_toplevel_children(
    scrim_parent: &mut ChildSpawnerCommands,
    root: Entity,
    title: &str,
    width: f32,
    height: f32,
) {
    // Window Card
    scrim_parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Px(width),
                min_height: Val::Px(height),
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(16.0),
                ..default()
            }),
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Large),
            ElevationToken(ElevationLevel::Level3),
        ))
        .with_children(|window_parent| {
            // Header Bar (Title + Close Button)
            window_parent
                .spawn(
                    (Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        ..default()
                    }),
                )
                .with_children(|header_parent| {
                    header_parent.spawn((
                        (
                            Text::new(title.to_string()),
                            TextFont {
                                font_size: FontSize::Px(20.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ),
                        ForegroundRole(ColorRole::OnSurface),
                        TypeStyleToken(TypeScale::TitleMedium),
                    ));

                    // Close Button
                    header_parent
                        .spawn((
                            (
                                Button,
                                Node {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            ToplevelCloseButton(root),
                            RadiusToken(RadiusLevel::Full),
                        ))
                        .with_children(|close_parent| {
                            close_parent.spawn((
                                (
                                    Text::new("✕"),
                                    TextFont {
                                        font_size: FontSize::Px(16.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ),
                                ForegroundRole(ColorRole::OnSurfaceVariant),
                                TypeStyleToken(TypeScale::LabelLarge),
                            ));
                        });
                });

            // Inner body container for custom contents
            window_parent.spawn((
                (Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    ..default()
                }),
                ToplevelBody(root),
            ));
        });
}

/// System handling clicks on the toplevel close button and despawning the window.
pub fn toplevel_close_system(
    mut commands: Commands,
    mut close_query: Query<(&ToplevelCloseButton, &Interaction), Changed<Interaction>>,
    mut events: MessageWriter<ToplevelClosed>,
) {
    for (btn, interaction) in &mut close_query {
        if *interaction == Interaction::Pressed {
            events.write(ToplevelClosed { entity: btn.0 });
            commands.entity(btn.0).despawn();
        }
    }
}

/// Plugin registering Toplevel widget systems and events.
#[derive(Default)]
pub struct ToplevelPlugin;

impl Plugin for ToplevelPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<ToplevelClosed>>() {
            app.add_message::<ToplevelClosed>();
        }
        app.add_systems(Update, toplevel_close_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toplevel_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            ToplevelBuilder::new("Settings Dialog")
                .modal(true)
                .spawn(&mut commands)
        };

        app.update();

        let toplevel = app.world().get::<M3Toplevel>(entity);
        assert_eq!(toplevel.map(|t| t.title.as_str()), Some("Settings Dialog"));
        assert_eq!(toplevel.map(|t| t.is_modal), Some(true));
    }
}
