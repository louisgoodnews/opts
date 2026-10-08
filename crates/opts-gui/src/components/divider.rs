//! Material Design 3 Divider widget for visual separation.

use bevy::prelude::*;

use crate::{theme::color::ColorRole, tokens::components::BackgroundRole};

/// Orientation of a divider.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Marker component for an M3 Divider root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Divider {
    pub orientation: DividerOrientation,
}

/// Builder for constructing Material Design 3 divider rules.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_my_divider(mut commands: Commands) {
///     DividerBuilder::horizontal()
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DividerBuilder {
    pub orientation: DividerOrientation,
    pub thickness: f32,
}

impl Default for DividerBuilder {
    fn default() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            thickness: 1.0,
        }
    }
}

impl DividerBuilder {
    /// Creates a horizontal divider builder with 1dp thickness.
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            thickness: 1.0,
        }
    }

    /// Creates a vertical divider builder with 1dp thickness.
    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            thickness: 1.0,
        }
    }

    /// Customizes divider line thickness in pixels.
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    fn style(&self) -> Style {
        match self.orientation {
            DividerOrientation::Horizontal => Style {
                width: Val::Percent(100.0),
                height: Val::Px(self.thickness),
                ..default()
            },
            DividerOrientation::Vertical => Style {
                width: Val::Px(self.thickness),
                height: Val::Percent(100.0),
                ..default()
            },
        }
    }

    /// Spawns the divider into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        commands
            .spawn((
                NodeBundle {
                    style: self.style(),
                    ..default()
                },
                M3Divider {
                    orientation: self.orientation,
                },
                BackgroundRole(ColorRole::OutlineVariant),
            ))
            .id()
    }

    /// Spawns the divider as a child inside a parent [`ChildBuilder`].
    pub fn spawn_child(self, parent: &mut ChildBuilder) -> Entity {
        parent
            .spawn((
                NodeBundle {
                    style: self.style(),
                    ..default()
                },
                M3Divider {
                    orientation: self.orientation,
                },
                BackgroundRole(ColorRole::OutlineVariant),
            ))
            .id()
    }
}

/// Plugin registering Divider widget systems.
#[derive(Default)]
pub struct DividerPlugin;

impl Plugin for DividerPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            DividerBuilder::horizontal().spawn(&mut commands)
        };

        app.update();

        let div = app.world().get::<M3Divider>(entity);
        assert_eq!(
            div.map(|d| d.orientation),
            Some(DividerOrientation::Horizontal)
        );

        let bg = app.world().get::<BackgroundRole>(entity);
        assert_eq!(bg.map(|b| b.0), Some(ColorRole::OutlineVariant));
    }
}
